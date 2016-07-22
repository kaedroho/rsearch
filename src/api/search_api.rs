use std::io::Read;

use rustc_serialize::json::{self, Json};
use url::form_urlencoded;

use system::System;
use search::query::parser::{QueryParseContext, parse as parse_query};
use search::index::reader::IndexReader;
use search::index::store::IndexStore;
use search::request::SearchRequest;

use api::persistent;
use api::iron::prelude::*;
use api::iron::status;
use api::router::Router;
use api::utils::json_response;


pub fn view_count(req: &mut Request) -> IronResult<Response> {
    let ref system = get_system!(req);
    let ref index_name = read_path_parameter!(req, "index").unwrap_or("");

    // Lock index array
    let indices = system.indices.read().unwrap();

    // Get index
    let index = get_index_or_404!(indices, *index_name);
    let index_reader = index.store.reader();

    let count = match json_from_request_body!(req) {
        Some(query_json) => {
            // Parse query
            let query = parse_query(&QueryParseContext::new().set_mappings(&index.mappings).no_score(), query_json.as_object().unwrap().get("query").unwrap());
            debug!("{:#?}", query);

            match query {
                Ok(query) => {
                    let request = SearchRequest {
                        query: query,
                        from: 0,
                        size: 0,
                        terminate_after: None,
                    };

                    request.run(&index_reader).total_hits
                }
                Err(error) => {
                    // TODO: What specifically is bad about the Query?
                    let mut response = Response::with((status::BadRequest,
                                                       "{\"message\": \"Query error\"}"));
                    response.headers.set_raw("Content-Type", vec![b"application/json".to_vec()]);
                    return Ok(response);
                }
            }
        }
        None => index_reader.num_docs(),
    };

    return Ok(json_response(status::Ok, format!("{{\"count\": {}}}", count)));
}


pub fn view_search(req: &mut Request) -> IronResult<Response> {
    let ref system = get_system!(req);
    let ref index_name = read_path_parameter!(req, "index").unwrap_or("");

    // Lock index array
    let indices = system.indices.read().unwrap();

    // Get index
    let index = get_index_or_404!(indices, *index_name);
    let index_reader = index.store.reader();

    match json_from_request_body!(req) {
        Some(query_json) => {
            // Parse query
            let query = parse_query(&QueryParseContext::new().set_mappings(&index.mappings), query_json.as_object().unwrap().get("query").unwrap());
            debug!("{:#?}", query);

            match query {
                Ok(query) => {
                    let mut request = SearchRequest {
                        query: query,
                        from: 0,
                        size: 10,
                        terminate_after: None,
                    };

                    // TODO: Rewrite this
                    if let Some(ref url_query) = req.url.query {
                        for (key, value) in form_urlencoded::parse(url_query.as_bytes()) {
                            match key.as_ref() {
                                "from" => {
                                    request.from = value.as_ref().parse().expect("need a number");
                                }
                                "size" => {
                                    request.size = value.as_ref().parse().expect("need a number");
                                }
                                "terminate_after" => {
                                    request.terminate_after = Some(value.as_ref()
                                                                        .parse()
                                                                        .expect("need a number"));
                                }
                                // explain
                                // version
                                // timeout
                                // fields
                                // fielddata_fields
                                // track_scores
                                // stats
                                // suggest_field
                                _ => warn!("unrecognised GET parameter {:?}", key),
                            }
                        }
                    }

                    let response = request.run(&index_reader);

                    // TODO: {"took":5,"timed_out":false,"_shards":{"total":5,"successful":5,"failed":0},"hits":{"total":4,"max_score":1.0,"hits":[{"_index":"wagtail","_type":"searchtests_searchtest_searchtests_searchtestchild","_id":"searchtests_searchtest:5380","_score":1.0,"fields":{"pk":["5380"]}},{"_index":"wagtail","_type":"searchtests_searchtest","_id":"searchtests_searchtest:5379","_score":1.0,"fields":{"pk":["5379"]}}]}}
                    Ok(json_response(status::Ok,
                                     format!("{{\"hits\": {{\"total\": {}, \"hits\": {}}}}}",
                                             response.total_hits,
                                             json::encode(&Json::Array(response.hits
                                                                               .iter()
                                                                               .map(|hit| {
                                                                                   hit.as_json()
                                                                               })
                                                                               .collect()))
                                                 .unwrap())))
                }
                Err(error) => {
                    // TODO: What specifically is bad about the Query?
                    let mut response = Response::with((status::BadRequest,
                                                       "{\"message\": \"Query error\"}"));
                    response.headers.set_raw("Content-Type", vec![b"application/json".to_vec()]);
                    Ok(response)
                }
            }
        }
        None => Ok(json_response(status::BadRequest, "{\"message\": \"Missing query\"}")),
    }
}
