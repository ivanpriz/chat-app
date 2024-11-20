use std::sync::mpsc::Receiver;

use rdkafka::producer::FutureProducer;
use tokio::sync::broadcast;

struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {
    fn pre_rebalance(&self, _: &BaseConsumer<Self>, rebalance: &Rebalance) {
        info!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, _: &BaseConsumer<Self>, rebalance: &Rebalance) {
        info!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(&self, result: KafkaResult<()>, _offsets: &TopicPartitionList) {
        info!("Committing offsets: {:?}", result);
    }
}

// A type alias with your custom consumer can be created for convenience.
type ChatConsumer = StreamConsumer<CustomContext>;

async fn create_consumer(brokers: &str, group_id: &str, topics: &[&str]) -> ChatConsumer {
    let context = CustomContext;

    ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        //.set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context)
        .expect("Consumer creation failed")
}

async fn create_producer(brokers: &str, topic_name: &str) -> FutureProducer {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error")
}

struct Chat {
    pub consumer: ChatConsumer,
    pub producer: FutureProducer,
    messages_sender: Arc<Mutex<Sender<String>>>,
    messages_receiver: Arc<Mutex<Receiver<String>>>,
}

impl Chat {
    pub fn new(consumer: ChatConsumer, producer: FutureProducer) -> Self {
        let (messages_sender, messages_receiver) = broadcast::channel(100);
        let (messages_sender, messages_receiver) = (
            Arc::new(Mutex::new(messages_sender)),
            Arc::new(Mutex::new(messages_receiver)),
        );
        Self {
            consumer,
            producer,
            messages_sender,
            messages_receiver,
        }
    }

    pub fn messages_sender(&self) -> Mutex<Sender<String>> {
        Arc::clone(&self.messages_sender)
    }

    pub fn messages_receiver(&self) -> Mutex<Receiver<String>> {
        Arc::clone(&self.messages_receiver)
    }
}
