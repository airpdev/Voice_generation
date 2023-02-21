use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::Consumer;
use rdkafka::ClientConfig;
use rdkafka::{
    consumer::{ConsumerContext, Rebalance, StreamConsumer},
    error::KafkaResult,
    ClientContext, TopicPartitionList,
};
pub struct CustomContext;

impl ClientContext for CustomContext {}

pub type LoggingConsumer = StreamConsumer<CustomContext>;

impl ConsumerContext for CustomContext {
    fn pre_rebalance(&self, rebalance: &Rebalance) {
        tracing::info!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, rebalance: &Rebalance) {
        tracing::info!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(&self, result: KafkaResult<()>, _offsets: &TopicPartitionList) {
        tracing::info!("Committing offsets: {:?}", result);
    }
}

pub fn get_consumer(
    brokers: &str,
    group_id: &str,
    topics: &[&str],
) -> StreamConsumer<CustomContext> {
    let context = CustomContext;
    let consumer: LoggingConsumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "10000")
        .set("enable.auto.commit", "true")
        .set("auto.commit.interval.ms", "1000")
        .set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context)
        .expect("Consumer creation failed");

    consumer
        .subscribe(&topics.to_vec())
        .expect("Can't subscribe to specified topics");

    return consumer;
}
