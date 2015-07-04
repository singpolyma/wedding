extern crate iron;
extern crate logger;
extern crate router;
extern crate urlencoded;

use iron::prelude::*;
use iron::status;
use logger::Logger;
use router::Router;
use urlencoded::UrlEncodedBody;

fn main() {
	fn rsvp(req: &mut Request) -> IronResult<Response> {
		Ok(Response::with(
			req.get_ref::<UrlEncodedBody>().ok().
				and_then({ |x| x.get("q") }).
				and_then({ |x| x.first() }).
				map_or(
					(status::BadRequest, "Invalid POST body.\n".to_string()),
					{ |q| (status::Ok, format!("{}\n", q)) }
				)
		))
	}

	let mut router = Router::new();
	router.post("/rsvp", rsvp);

	let mut chain = Chain::new(router);
	chain.link(Logger::new(None));

	Iron::new(chain).http("localhost:3000").unwrap();

	println!("Started...");
}
