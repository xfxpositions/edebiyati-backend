use rusoto_core::credential::{StaticProvider};
use rusoto_core::Region;
use rusoto_s3::{PutObjectRequest, S3Client, S3};
use dotenv::dotenv;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use percent_encoding::{AsciiSet, CONTROLS};

use std::env;
use std::error::Error;

pub async fn upload_image_to_s3(key: &str, data: &[u8]) -> Result<String, Box<dyn Error>> {
    dotenv().ok();
    // Read the access key and secret key from the .env file
    let access_key = env::var("AWS_ACCESS_KEY_ID")?;
    let secret_key = env::var("AWS_SECRET_ACCESS_KEY")?;
    let bucket_name = env::var("AWS_BUCKET_NAME")?;

    // Create a S3 client using the provided credentials and region
    let credentials_provider = StaticProvider::new_minimal(access_key, secret_key);
    let region = Region::EuCentral1;
    let s3_client = S3Client::new_with(
        rusoto_core::HttpClient::new().unwrap(),
        credentials_provider,
        region.clone(),
    );

    // Encode the object key
    let encoded_key = utf8_percent_encode(key, NON_ALPHANUMERIC).to_string();

    // Create a PutObjectRequest for the image data
    let put_request = PutObjectRequest {
        bucket: bucket_name.to_owned(),
        key: key.clone().to_owned(),
        body: Some(data.to_vec().into()),
        ..Default::default()
    };

    // Upload the image data to S3
    match s3_client.put_object(put_request).await {
        Ok(response) => {
            let url = format!("https://{}.s3.{}.amazonaws.com/{}", bucket_name, region.name(), key);
                // Define a set of characters that should be encoded
                const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

                // Encode the URL string
                let encoded_url = utf8_percent_encode(&url, FRAGMENT).to_string();
            Ok(encoded_url)
        },
        Err(err) => Err(Box::new(err)),
    }
}