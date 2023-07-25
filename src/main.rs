use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server, Uri};
use hyper::service::{make_service_fn, service_fn};

async fn handler(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let (parts, body) = _req.into_parts();
    let static_files = vec![".ico", ".css", ".js", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ttf", ".woff", ".woff2", ".eot", ".otf", ".html"];
    for file in static_files {
        if parts.uri.to_string().contains(file) {
            // Get rid of any search parameters
            let uri = parts.uri.to_string();
            let url = uri.split("?").collect::<Vec<&str>>()[0];
            let mut path = "".to_string();
            if parts.uri.to_string().starts_with("/phpmyadmin") {
                let mut executable_path = env::current_exe().unwrap();
                executable_path.pop();
                path = format!("{}{}{}", executable_path.to_str().unwrap().to_string(), "", url);
            }
            else {
                path = format!("{}{}", std::env::current_dir().unwrap().to_str().unwrap(), url);
            }
            match std::fs::read(&path) {
                Ok(contents) => return Ok(Response::new(contents.into())),
                Err(_) => {
                    println!("404: {} | Path: {}", parts.uri, &path);
                    return Ok(Response::builder()
                        .status(404)
                        .body(Body::from("404 Not Found"))
                        .unwrap());
                }
            }
        }
    }

    // Route requests through to the php server
    let client = hyper::Client::new();
    // If the uri starts with "/phpmyadmin", than change the port to 1112
    let mut port = 1111;
    if parts.uri.to_string().starts_with("/phpmyadmin") {
        port = 1112;
    }
    let str_uri = format!("http://127.1.1.1:{}{}", port, parts.uri.to_string());
    let uri: Uri = str_uri.parse().unwrap();
    let mut req = Request::builder()
        .method(parts.method)
        .uri(uri);
    let headers = req.headers_mut().unwrap();
    // Add the headers
    for (key, value) in parts.headers.iter() {
        headers.append(key, value.to_owned());
    }
    // Add the body
    let req = req.body(Body::from(body)).unwrap();

    let res = client.request(req).await.expect("Failed to pass response on... ");

    Ok(res)
}

#[tokio::main]
async fn main() {
    // We'll bind to 127.0.0.1:3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // A `Service` is needed for every connection, so this
    // creates one from our `hello_world` function.
    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handler))
    });

    let server = Server::bind(&addr).serve(make_svc);

    let work_dir = std::env::current_dir().unwrap();
    let work_dir = work_dir.to_str().unwrap();
    let mut executable_path = env::current_exe().unwrap();
    // Strip the executable name from the path
    executable_path.pop();

    println!("Executable path: {}", executable_path.to_str().unwrap());
    println!("Working directory: {}", &work_dir);

    // Start the php in-built webserver
    let php_file = format!("{}{}", executable_path.to_str().unwrap(), "/php/php");
    let mut _php_server = std::process::Command::new(&php_file)
        .args(&[
            "-S",
            "127.1.1.1:1111",
        ])
        .current_dir(&work_dir)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let mut _pma_server = std::process::Command::new(&php_file)
        .args(&[
            "-S",
            "127.1.1.1:1112",
        ])
        .current_dir(&executable_path)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
 
    let mysql_file = format!("{}{}", executable_path.to_str().unwrap(), "/mysql/bin/mysqld");

    // If mysql/data doesn't exist, create it
    let mysql_data_path = format!("{}{}", executable_path.to_str().unwrap(), "/mysql/data");
    if !std::path::Path::new(&mysql_data_path).exists() {
        // Run mysqld --initialize-insecure --user=root --console
        println!("Initializing MySQL... Please restart the server after this is done.");
        let _mysql_init = std::process::Command::new(&mysql_file)
            .args(&[
                "--initialize-insecure",
                "--user=root",
                "--console",
            ])
            .current_dir(&executable_path)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
    }

    let mut _mysql_server = std::process::Command::new(mysql_file)
            .current_dir(&executable_path)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}