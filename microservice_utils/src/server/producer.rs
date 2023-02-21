use std::time::Duration;

use rdkafka::{
    error::KafkaError,
    message::OwnedHeaders,
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};

// Get the producer from redpand kafka
pub fn get_producer(brokers: &str) -> FutureProducer {
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("message.timeout.ms", "5000")
        .set("debug", "all")
        .create()
        .expect("Producer creation failed");

    producer
}

// send message to rdkafka topic
pub async fn produce(
    json_message: &String,
    producer: &FutureProducer, 
    topic: &str,
) -> Result<(i32, i64), (KafkaError, rdkafka::message::OwnedMessage)> {
    let delivery_status = producer
        .send(
            FutureRecord::to(topic)
                .payload(&json_message)
                .key(&format!("{}", uuid::Uuid::new_v4().to_string()))
                .headers(OwnedHeaders::default()),
            Duration::from_secs(1),
        )
        .await;
    // This will be executed when the result is received.
    delivery_status
}
