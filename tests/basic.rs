use futures::Future;
use enande::Processor;

mod common;

pub struct Error {
    pub message: String,
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
	Error { message: format!("This did not happen!") }
    }
}

struct Simple;

impl enande::Processor for Simple {
    type Item = String;
    type Error = Error;
    type ResultItem = String;
    type ResultFuture = Box<dyn Future<Output = Result<Self::ResultItem, Self::Error>> + Unpin>;


    fn process(&mut self, _: Self::Item) -> Self::ResultFuture {
	let res = futures::future::ready(Ok("A String".to_owned()));
	Box::new(res)
    }

    
}

#[test]
fn test_som_spammers() {
    let mut b = Simple::builder();
    let mut barrel = common::barrel();


    // Tried 1_000 different sources each sending 1_000 messages (10_000_000 msgs).
    // Handled in 4.5s

    for i in 0..100 {
	b.add_stream(common::spammer(format!("Spammer #{}", i), 100));
    }

    let fut = b.run(barrel.pipe(), Simple);
    let mut rt = tokio::runtime::current_thread::Runtime::new().expect("Creating runtime");


    rt.block_on(fut);

    assert_eq!(100*100, barrel.collect().len());
}



