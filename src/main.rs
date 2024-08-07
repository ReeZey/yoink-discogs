use std::{fs::File, io::Cursor, thread::sleep, time::Duration};

use serde_json::{json, Value};
use image::ImageReader;

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
                
                buffer = response.bytes().unwrap();
                
                if image_resp == 200 {
                    println!("we all good, lets continue");
                    break;
                }
                println!("response error: {:?}", image_resp);
                if idx == 4 {
                    let bytes: Vec<u8> = vec![];
                    buffer = bytes.into();
                }
            }
            buffer
        } else {
            response.bytes().unwrap()
        };

        let img = ImageReader::new(Cursor::new(bytes)).with_guessed_format().unwrap().decode().unwrap();

        let file_name = format!("images/{}-{}.jpeg", urlencoding::encode(&title), urlencoding::encode(&artists));

        img.save(&file_name).unwrap();

        let data = json!({
            "date_added": date_added,
            "title": title,
            "year": year,
            "artists": artists,
            "genres": genres,
            "styles": styles,
            "formats": formats,
            "image": file_name
        });
        output.push(data);

        println!("{} [{}/{}]", title, idx, releases.len());
    }

    let file = File::create("output.json").unwrap();
    serde_json::to_writer(file, &output).unwrap();
}
