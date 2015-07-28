extern crate iron;
extern crate logger;
extern crate router;
extern crate urlencoded;
extern crate sqlite3;
extern crate rustache;

use iron::prelude::*;
use iron::status;
use logger::Logger;
use router::Router;
use urlencoded::UrlEncodedBody;
use sqlite3::{Query, ResultRowAccess};

fn main() {

	fn rsvp(req: &mut Request) -> IronResult<Response> {
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();

		Ok(Response::with(
			req.get_ref::<UrlEncodedBody>().ok().
				and_then( |x| x.get("q") ).
				and_then( |x| x.first() ).
				map_or(
					(status::BadRequest, "Invalid POST body.\n".to_string()),
					|q| {
						Vec::new().insert_vector("guests", |guests|
							db.prepare("SELECT * FROM guests WHERE fn LIKE $1").unwrap().
								query(&[&format!("%{}%", q)], &mut |row|
									guests.push_hash( |guest|
										guest.insert_string("fn", row.get("fn"))
									)
								).unwrap();
						);
						(status::Ok, format!("{}\n", q))
					}
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
