use bollard::Docker;
use bollard::container::UploadToContainerOptions;
use bollard::image::BuildImageOptions;
use futures_util::stream::StreamExt;
use url::Url;

use std::env;
use std::fs::File;
use std::path::Path;

pub struct Client {
    inner: Docker,
    host: String
}

impl Client {
    pub fn new() -> Self {
        let host = env::var("DOCKER_HOST").ok()
            .map(|ref h| Url::parse(h).expect(&format!("Can't parse DOCKER_HOST: {h}")));

        match host {
            Some(x) if !matches!(x.scheme(), "unix" | "npipe") => Self {
                inner: Docker::connect_with_http_defaults().unwrap(),
                host: x.host_str().unwrap().to_string()
            },
            Some(x) => Self {
                inner: Docker::connect_with_socket(x.path(), 60, bollard::API_DEFAULT_VERSION).unwrap(),
                host: Self::socket_host()
            },
            None => Self {
                inner: Docker::connect_with_socket_defaults().unwrap(),
                host: Self::socket_host()
            }
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub async fn build_local_image(
        &self, image: &str, tag: &str, mut dockerfile: File) -> Result<(), bollard::errors::Error> {

        let mut tarball: Vec<u8> = Vec::new();
        tar::Builder::new(&mut tarball).append_file(
            "Dockerfile",
            &mut dockerfile
        ).expect("Can't make tarball");

        let name = format!("{image}:{tag}");
        let mut stream = self.inner.build_image(
            BuildImageOptions {
                dockerfile: "Dockerfile",
                t: &name,
                cachefrom: vec![&name],
                forcerm: true,
                ..Default::default()
            },
            None, 
            Some(tarball.into())
        );

        while let Some(res) = stream.next().await {
            match res {
                Ok(x) => { x.stream.filter(|s| s != "\n").map(|s| println!("Building image...{s}")); },
                Err(e) => return Err(e)
            }
        };
        return Ok(());
    }

    pub async fn copy_file(
        &self, container_id: &str, mut src_file: File, dest_path: &Path) -> Result<(), bollard::errors::Error> {

        let mut tarball: Vec<u8> = Vec::new();
        tar::Builder::new(&mut tarball).append_file(
            dest_path.file_name().and_then(|x| x.to_str()).unwrap(),
            &mut src_file
        ).expect("Can't make tarball");

        self.inner.upload_to_container(
            container_id, 
            Some(UploadToContainerOptions {
                path: dest_path.parent().and_then(|x| x.as_os_str().to_str()).unwrap(),
                no_overwrite_dir_non_dir: "true",
             }),
             tarball.into()
        ).await
    }

    fn socket_host() -> String {
        if Path::new("/.dockerenv").is_file() {
            Self::docker_in_docker_host().unwrap()
        } else {
            "localhost".to_string()
        }
    }
    fn docker_in_docker_host() -> anyhow::Result<String> {
        use std::{process::Command, str::from_utf8};
    
        let cmd = Command::new("sh")
            .arg("-c")
            .arg("ip route|awk '/default/ { print $3 }'")
            .output()?;
        
        let ip = from_utf8(&cmd.stdout)?;
        Ok(ip.trim().to_string())
    }
}
