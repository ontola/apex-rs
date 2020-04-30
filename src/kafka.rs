use crate::delta_stream::DeltaMessage;
use futures::stream::Next;
use futures::StreamExt;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::{Consumer, DefaultConsumerContext, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::ClientConfig;

struct KafkaDeltaMessage<'a, 'f: 'a> {
    consumer: &'a BorrowedMessage<'f>,
    message: &'a DefaultConsumerContext,
}

impl<'a, 'f: 'a> DeltaMessage for KafkaDeltaMessage<'a, 'f> {
    fn read(&self) {
        self.message;
    }

    fn finish(&self) {
        if let Err(e) = self.consumer.store_offset(self.message) {
            warn!("Error while storing offset: {}", e);
        }
    }
}

async fn create_kafka_stream<'a>() -> Box<dyn StreamExt<Item = DeltaMessage>> {
    let consumer = create_kafka_consumer();
    println!("Initialized kafka config");

    let topic = dotenv!("KAFKA_TOPIC");
    consumer
        .subscribe(&[topic])
        .expect("Cannot subscribe to topic");

    consumer
}

fn create_kafka_consumer() -> Box<StreamConsumer> {
    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", dotenv!("KAFKA_ADDRESS"));

    config.set("sasl.mechanisms", "PLAIN");
    config.set("security.protocol", "SASL_SSL");
    config.set("sasl.username", dotenv!("KAFKA_USERNAME"));
    config.set("sasl.password", dotenv!("KAFKA_PASSWORD"));

    config.set("group.id", dotenv!("KAFKA_GROUP_ID"));
    config.set("queue.buffering.max.ms", "0");
    //    config.set("allow.auto.create.topics", "false");
    config.set("fetch.max.bytes", "5428800");
    //    config.set("fetch.max.wait.ms", "1000");
    //    config.set("max.poll.records", "500");
    config.set("session.timeout.ms", "60000");
    config.set("enable.auto.commit", "true");
    config.set("auto.commit.interval.ms", "1000");
    config.set("request.timeout.ms", "20000");
    config.set("retry.backoff.ms", "500");

    config.set_log_level(RDKafkaLogLevel::Debug);

    stream = config
        .create::<StreamConsumer>()
        .expect("Failed to create kafka consumer");

    Box::new(stream)
}
