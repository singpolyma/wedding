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
use urlencoded::{UrlEncodedBody, UrlEncodedQuery};
use sqlite3::{Query, ResultRowAccess, StatementUpdate};
use sqlite3::types::{ToSql};
use sqlite3::core::{ResultRow, PreparedStatement};
use rustache::HashBuilder;

fn option_map_mut<T, F: FnOnce(T) -> T>(x: &mut Option<T>, f: F) {
	match *x {
		Some(_) => {
			*x = Some(f(x.take().unwrap()));
		}
		None => {}
	}
}

fn query_fold<T, F: Fn(&mut ResultRow, T) -> T>(z: T, q: &mut PreparedStatement, values: &[&ToSql], f: F) -> T
{
	let mut zbox = Some(z);

	q.query(values, &mut |row| {
		option_map_mut(&mut zbox, { |z| f(row, z) });
		Ok(())
	}).unwrap();

	zbox.unwrap()
}

fn main() {

	fn rsvp_search(req: &mut Request) -> IronResult<Response> {
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();

		Ok(Response::with(
			req.get_ref::<UrlEncodedQuery>().ok().
				and_then( |x| x.get("q") ).
				and_then( |x| x.first() ).
				map_or(
					(iron::modifiers::Header(iron::headers::ContentType::plaintext()), status::BadRequest, "Invalid POST body.\n".to_string().into_bytes()),
					|q|
						(iron::modifiers::Header(iron::headers::ContentType::html()), status::Ok, rustache::render_file("views/rsvp_results.mustache", HashBuilder::new().insert_vector("guests", |guests| {
							query_fold(
								guests,
								&mut db.prepare("SELECT * FROM guests WHERE fn LIKE ?1 OR email LIKE ?1 OR search_name LIKE ?1").unwrap(),
								&[&format!("%{}%", q)],
								|row, vec| {
									let name = row.get::<&str, String>("fn");
									let id = row.get::<&str, i32>("id");
									vec.push_hash( |guest|
										guest.
											insert_string("fn", name.clone()).
											insert_int("id", id)
									)
								}
							)
						})).unwrap().unwrap())
				)
		))
	}

	fn rsvp_form(req: &mut Request) -> IronResult<Response> {
		let guestid : i32 = req.extensions.get::<Router>().unwrap().find("guestid").unwrap().parse().unwrap();
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();

		Ok(Response::with(
			(
				iron::modifiers::Header(iron::headers::ContentType::html()),
				status::Ok,
				rustache::render_file("views/rsvp_form.mustache",
					query_fold(
						HashBuilder::new(),
						&mut db.prepare("SELECT * FROM guests WHERE id=$1").unwrap(),
						&[&guestid],
						|row, hsh| {
							hsh.insert_string("fn", row.get::<&str, String>("fn"))
						}
					)
				).unwrap().unwrap()
			)
		))
	}

	fn rsvp(req: &mut Request) -> IronResult<Response> {
		let guestid : i32 = req.extensions.get::<Router>().unwrap().find("guestid").unwrap().parse().unwrap();
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();

		let body = req.get_ref::<UrlEncodedBody>().ok();
		let coming = body.and_then( |x| x.get("coming") ).and_then( |x| x.first() ).map( |x| x == "coming" ).unwrap_or(false);
		let vegetarian = body.and_then( |x| x.get("vegetarian") ).and_then( |x| x.first() ).and_then( |x| x.parse().ok() ).unwrap_or(0);
		let carpool = body.and_then( |x| x.get("carpool") ).and_then( |x| x.first() ).map( |x| x.clone() );
		let songs = body.and_then( |x| x.get("songs") ).and_then( |x| x.first() ).map( |x| x.clone() );
		let note = body.and_then( |x| x.get("note") ).and_then( |x| x.first() ).map( |x| x.clone() );

		db.prepare("UPDATE guests SET coming=?2, vegetarian=?3, carpool=?4, songs=?5, note=?6 WHERE id=?1").unwrap().
			update(&[&guestid, &coming, &vegetarian, &carpool, &songs, &note]).
			unwrap();

		Ok(Response::with(
			(
				iron::modifiers::Header(iron::headers::ContentType::html()),
				status::Ok,
				rustache::render_file("views/rsvp_done.mustache", HashBuilder::new()).unwrap().unwrap()
			)
		))
	}

	let mut router = Router::new();
	router.get("/rsvp", rsvp_search);
	router.get("/rsvp/:guestid", rsvp_form);
	router.post("/rsvp/:guestid", rsvp);

	let mut chain = Chain::new(router);
	chain.link(Logger::new(None));

	println!("Starting on port 3000...");
	Iron::new(chain).http("localhost:3000").unwrap();
}
