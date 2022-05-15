extern crate core;

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use clap::{Arg, ArgMatches, Command};
use reqwest::{Request, Response, StatusCode};
use reqwest::header::{HeaderMap};
use colored::Colorize;

fn print_req(req: &Request) {
    println!(
        "> {} {:?} {}",
        req.method(),
        req.version(),
        req.url().path()
    );
    println!("> Host: {}", req.url().host().unwrap());
    let req_headers = req.headers();
    for (k, v) in req_headers {
        println!("> {}: {}", k, v.to_str().unwrap())
    }
    println!(">")
}

fn highlight_status_code(code: &StatusCode) -> String {
    if code.to_string().starts_with('2') {
        return format!("{}", code.to_string().green());
    } else if code.to_string().starts_with('3') {
        return format!("{}", code.to_string().yellow());
    } else if code.to_string().starts_with('4') {
        return format!("{}", code.to_string().red());
    } else if code.to_string().starts_with('5'){
        return format!("{}", code.to_string().red().bold());
    } else {
        String::new()
    }
}

fn print_res(res: &Response) {
    println!(
        "< {:?} {}", res.version(), highlight_status_code(&res.status())
    );
    let res_headers = res.headers();
    for (k, v) in res_headers {
        println!("< {}: {}", k, v.to_str().unwrap());
    }

    println!("<")
}

fn parse_headers(matches: &ArgMatches) -> HeaderMap {
    if matches.is_present("header") {
        return HeaderMap::new();
    }
    let mut header_map = HashMap::new();
    let headers: Vec<&str> = matches.values_of("header").unwrap_or_default().collect();

    for header in headers {
        let values: Vec<&str> = header.split(':').collect();
        if values.len() != 2 {
            panic!("Unexpected header format {}", header);
        }
        let k = values[0].to_string().to_lowercase();
        let v = values[1].trim_end().to_string();
        header_map.insert(k, v);
    }
    (&header_map).try_into().expect("Invalid headers")
}

fn parse_fields(matches: &ArgMatches) -> HashMap<String, String> {
    if !matches.is_present("form") {
        return HashMap::new();
    }
    let mut header_map = HashMap::new();
    let fields: Vec<&str> = matches.values_of("form").unwrap_or_default().collect();
    for field in fields {
        let values: Vec<&str> = field.split("=").collect();
        if values.len() != 2 {
            panic!("Unexpected form format {}", field)
        }
        let k = values[0].to_string();
        let v = values[1].trim_start().to_string();

        header_map.insert(k, v);
    }
    header_map
}

fn parse_data(matches: &ArgMatches) -> String {
    if !matches.is_present("data") {
        return String::new();
    }
    let fields: Vec<&str> = matches.values_of("data").unwrap_or_default().collect();
    fields.join("&").to_string()
}

async fn save_in_file(out_path: PathBuf, data: String) -> Result<(), io::Error>{
    let mut file = File::create(out_path)?;
    file.write_all(data.as_bytes())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let matches = Command::new(
        env!("CARGO_PKG_NAME")
    ).version(
        env!("CARGO_PKG_VERSION")
    ).about("Cli tool that makes request to the endpoints and processes the responses")
        .author("BufferOverflow")
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Sets the output level to verbose")
        )
        .arg(
            Arg::new("method")
                .short('X')
                .long("method")
                .takes_value(true)
                .possible_values(&["POST", "GET", "PUT", "PATCH", "HEAD", "DELETE"])
                .ignore_case(true)
                .help("Sets the http method for the request")
        )
        .arg(
            Arg::new("header")
                .short('H')
                .multiple_values(true)
                .takes_value(true)
                .help("Sets header content for the request")
        )
        .arg(
            Arg::new("form")
                .short('F')
                .takes_value(true)
                .multiple_values(true)
                .help("Set the form values in a field=value pair")
        )
        .arg(
            Arg::new("data")
                .short('d')
                .multiple_values(true)
                .takes_value(true)
                .help("Sets the data values and combine from a field=value pair")
        )
        .arg(
            Arg::new("uri")
                .index(1)
                .required(true)
        )
        .arg(
            Arg::new("out")
                .value_name("PATH")
                .short('o')
                .long("out-path")
                .help("Saves the response in the file")
        ).get_matches();

    let uri = matches.value_of("uri").unwrap();

    let client = reqwest::Client::new();

    let method = matches.value_of("method").unwrap();

    let req_builder = match method {
        "GET" => client.get(uri),
        "POST" | "PUT" | "PATCH" => {
            let b = match method {
                "PUT" => client.put(uri),
                "PATCH" => client.patch(uri),
                _ => client.post(uri),
            };
            if matches.is_present("form") {
                b.form(&parse_fields(&matches))
            } else if matches.is_present("data"){
                b.body(parse_data(&matches))
            } else {
                b
            }
        }
        "HEAD" => client.head(uri),
        "DELETE" => client.delete(uri),
        _ => panic!("Invalid method")
    };

    let req = req_builder
        .headers(parse_headers(&matches))
        .build()
        .unwrap();

    if matches.is_present("verbose") {
        print_req(&req);
    }

    let response = client.execute(req).await?;

    if matches.is_present("verbose") {
        print_res(&response);
    }


    let text = response.text().await?;

    if matches.is_present("out") {
        if let Some(path_str) = matches.value_of("out") {
            println!("Saving...");
            save_in_file(PathBuf::from(path_str), text).await.expect("Could not save the file");
            println!("Saved response text in {}", path_str)
        }
    } else {
        println!("{}", text.trim_end());
    }

    Ok(())
}



