extern crate iron;
extern crate logger;
extern crate router;

use iron::prelude::*;
use iron::status;
use logger::Logger;
use router::Router;

fn main() {
	fn hello_world(_: &mut Request) -> IronResult<Response> {
		Ok(Response::with((status::Ok, "Hello World!")))
	}

	let mut router = Router::new();
	router.get("/", hello_world);

	let mut chain = Chain::new(router);
	chain.link(Logger::new(None));

	Iron::new(chain).http("localhost:3000").unwrap();

	println!("Started...");
}
