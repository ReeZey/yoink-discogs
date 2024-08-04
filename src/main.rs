use base64::prelude::*;
use std::{fs::File, thread::sleep, time::Duration};

use serde_json::{json, Value};

fn main() {
    dotenvy::dotenv().unwrap();
    let web_client = reqwest::blocking::Client::new();

    let response = web_client
        .get(std::env::var("API_PATH").unwrap())
        .query(&[
            ("token", std::env::var("DISCOGS_TOKEN").unwrap()),
            ("per_page", "500".to_string()),
        ])
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0",
        )
        .send()
        .unwrap();

    let json: Value = response.json().unwrap();

    let mut output: Vec<Value> = Vec::new();

    let releases = json["releases"].as_array().unwrap();
    for (idx, release) in releases.iter().enumerate() {
        let date_added = release["date_added"].as_str().unwrap();
        let title = release["basic_information"]["title"].as_str().unwrap();
        let year = release["basic_information"]["year"].as_i64().unwrap();

        let artists = release["basic_information"]["artists"]
            .as_array()
            .unwrap()
            .iter()
            .map(|artist| artist["name"].as_str().unwrap())
            .collect::<Vec<&str>>()
            .join(", ");
        let genres = release["basic_information"]["genres"]
            .as_array()
            .unwrap()
            .iter()
            .map(|genre| genre.as_str().unwrap())
            .collect::<Vec<&str>>()
            .join(", ");
        let styles = release["basic_information"]["styles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|style| style.as_str().unwrap())
            .collect::<Vec<&str>>()
            .join(", ");

        let formats = release["basic_information"]["formats"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|format| {
                let formatted = format!(
                    "{} ({})",
                    format["name"].as_str().unwrap(),
                    format["descriptions"]
                        .as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(|descriptor| descriptor.as_str().unwrap())
                        .collect::<Vec<&str>>()
                        .clone()
                        .join(", ")
                );

                match format["name"].as_str().unwrap() {
                    "CD" | "Vinyl" | "Cassette" | "DVD" | "VHS" => Some(formatted),
                    _ => return None,
                }
            })
            .collect::<Vec<String>>()
            .join(", ");

        let image = release["basic_information"]["cover_image"]
            .as_str()
            .unwrap();

        let response = reqwest::blocking::get(image).unwrap();
        let image_resp = response.status();

        let bytes = if image_resp != 200 {
            let mut buffer = response.bytes().unwrap();
            for idx in 0..5 {
                println!("need to wait ...");
                sleep(Duration::from_secs(65));
                let response = reqwest::blocking::get(image).unwrap();

                let image_resp = response.status();
                println!("{:?}", image_resp);

                buffer = response.bytes().unwrap();

                if image_resp == 200 {
                    break;
                }
                if idx == 4 {
                    let bytes: Vec<u8> = vec![];
                    buffer = bytes.into();
                }
            }
            buffer
        } else {
            response.bytes().unwrap()
        };

        let base_64 = BASE64_STANDARD.encode(bytes.as_ref());

        let data = json!({
            "date_added": date_added,
            "title": title,
            "year": year,
            "artists": artists,
            "genres": genres,
            "styles": styles,
            "formats": formats,
            "image": format!("data:image/jpeg;base64,{}", base_64)
        });
        output.push(data);

        println!("{} [{}/{}]", title, idx, releases.len());
    }

    let file = File::create("output.json").unwrap();
    serde_json::to_writer(file, &output).unwrap();
}
