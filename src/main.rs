use std::{env, sync::Arc};

use megalodon::{entities::UploadMedia, mastodon::Mastodon, Megalodon};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufStream},
    net::TcpStream,
};
use tokio_rustls::{client::TlsStream, rustls};

#[tokio::main]
async fn main() {
    let mastodon = Mastodon::new(
        env::var("MASTODON_BASE_URL").unwrap(),
        Some(env::var("MASTODON_TOKEN").unwrap()),
        None,
    );

    let connection = connect().await;

    let media = mastodon
        .upload_media_reader(Box::new(connection), None)
        .await
        .unwrap();

    let media_id;
    if let UploadMedia::Attachment(media) = media.json {
        media_id = media.id;
    } else if let UploadMedia::AsyncAttachment(a) = media.json {
        for _ in 0..10 {
            match mastodon.get_media(a.id.clone()).await {
                Ok(media) => {
                    media_id = media.json.id;
                    break;
                }
                Err(_) => tokio::time::sleep(std::time::Duration::from_secs(10)).await,
            };
        }
        panic!("failed to upload");
    } else {
        panic!("something went wrong");
    }

    let options = megalodon::megalodon::PostStatusInputOptions {
        media_ids: Some(vec![media_id]),
        ..Default::default()
    };

    mastodon
        .post_status("test".to_string(), Some(&options))
        .await
        .unwrap();
}

async fn connect() -> BufStream<TlsStream<TcpStream>> {
    let host = "pbs.twimg.com";
    let port = "443";
    let host_port = format!("{}:{}", host, port);

    let mut certs = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().unwrap() {
        certs.add(&rustls::Certificate(cert.0)).unwrap();
    }

    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(certs)
        .with_no_client_auth();
    let config = tokio_rustls::TlsConnector::from(Arc::new(config));

    let connection = TcpStream::connect(host_port).await.unwrap();
    let connection = config
        .connect(host.try_into().unwrap(), connection)
        .await
        .unwrap();
    let mut connection = BufStream::new(connection);

    connection.write_all(b"GET /media/EFijGAXXkAAGQ2q?format=jpg&name=4096x4096 HTTP/1.0\r\nHost: pbs.twimg.com\r\nConnection: close\r\n\r\n").await.unwrap();
    connection.flush().await.unwrap();

    let mut buf = String::new();
    while &buf != "\r\n" {
        buf.clear();
        connection.read_line(&mut buf).await.unwrap();
    }

    connection
}
