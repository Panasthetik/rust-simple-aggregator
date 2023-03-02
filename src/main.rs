// General async / serde / std dependencies
use std::error::Error;
use tokio;
use futures::stream::{StreamExt};
use serde::{Serialize, Deserialize};

// mongoDB specific dependencies:
use mongodb::options::{ClientOptions, ResolverConfig};
use mongodb::bson::{doc, oid::ObjectId};
use mongodb::Collection;

// Supabase dependencies:
use postgrest::Postgrest;
use dotenv::dotenv;
use serde_json::{json, Value};
use serde::ser::StdError;

// Near Protocol dependencies:
use near_jsonrpc_client::methods;
use near_jsonrpc_client::JsonRpcClient;
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::types::{AccountId, BlockReference, Finality};
use near_primitives::views::QueryRequest;

// MongoDB Structs:
#[derive(Serialize, Deserialize, Debug)]
struct Movie {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    title: String,
    cast: String,
    year: i32,
    plot: String,
    // #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    // released: chrono::DateTime<Utc>,
}
// MongoDB Summary Struct
#[derive(Debug, Deserialize)]
struct YearSummary {
    _id: i32,
    #[serde(default)]
    movie_count: i64,
    #[serde(default)]
    movie_titles: Vec<String>,
}

// Supabase struct:
#[derive(Debug, Serialize, Deserialize)]
pub struct Employee {
    id: i8,
    first_name: String,
    age: i8,
    interests: String,
    city: String
}

// Main function without special formatting or tokio::spawn :
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let output_1 = 
        near_get_account().await;
    let output_2 = 
        supabase_get_employees().await;
    let output_3 = 
        mongo_db_get().await;

    println!("{:?}, {:?}, {:?}", output_1, output_2, output_3);

// To add: concurrency, narrowing filters and formatting for output.

    Ok(())
}

// Near Protocol get account:
async fn near_get_account() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // let client = utils::select_network()?;
    let client = JsonRpcClient::connect("https://rpc.testnet.near.org".to_string());

    // let account_id: AccountId = utils::input("Enter an Account ID to lookup: ")?.parse()?;
    let account_id: AccountId = "panasthetik.testnet".parse().unwrap();

    let cloned_account = account_id.clone();

    let request = methods::query::RpcQueryRequest {
        block_reference: BlockReference::Finality(Finality::Final),
        request: QueryRequest::ViewAccount { account_id },
    };

    let response = client.call(request).await?;

    if let QueryResponseKind::ViewAccount(result) = response.kind {
        println!("Account: {} // Yocto: {:#?}", cloned_account, result.amount);
    }

    Ok(())
}

// Supabase get employees data
async fn supabase_get_employees() -> Result<(), Box<dyn Error>> {
    dotenv::from_filename(".env").ok();
    // postgrest client
    let client = Postgrest::new("SUPABASE_URI")
    .insert_header("apikey", dotenv::var("SUPABASE_KEY").unwrap());
    
    // simple SQL wildcard query for all results in Employees..
    let resp = client
    .from("employees")
    .select("*")
    .execute()
    .await?;

    // objects are results in JSON format
    let objects = resp
    .text()
    .await?;
    // println!("{}", serde_json::to_string_pretty(&objects).unwrap());

    // puts all employees in a vec result:
    let employees: Vec<Employee> = serde_json::from_str(&objects)?;

    // iterates thru the vec individual employees w/ desired fields
    for result in employees {
    println!("Employee: {} // Age: {}, // Interests: {} // City: {} //",
        result.first_name,
        result.age,
        result.interests,
        result.city
    );
}

    Ok(())
}

// is async version of the client implementation
async fn mongo_db_get() -> Result<(), Box<dyn Error>> {
    dotenv::from_filename(".env").ok();
    // fetches URI from env
    let client_uri = dotenv::var("MONGODB_URI").unwrap();
    let options = 
        ClientOptions::parse_with_resolver_config(
            &client_uri,
            ResolverConfig::cloudflare())
            .await?;

    let client = mongodb::Client::with_options(options)?;
    // uses sample movie database to ping server:
    let db = client.database("sample_mflix");
    db.run_command(doc! {"ping": 1}, None).await?;
    println!("Connected to movies database successfully");

    // stage 2: query the movies list first ten:
  
    let movies: Collection<Movie> = db.collection("movies");

    // group the movies by year released as ID;
    let stage_filter_valid_years = doc! {
        "$match": {
            "year": {
                "$type": "number",
            }
        }
    };
    // movie count and title
    let stage_group_year = doc! {
        "$group": {
            "_id": "$year",
            "movie_count": { "$sum": 1 },
            "movie_titles": { "$push": "$title" },
        }
    };
    // sort by year ascending
    let stage_sort_year_ascending = doc! {
        "$sort": {"_id": 1}
    };
    // limit 15 "year" entries
    let limit = doc! {
        "$limit": 10
    };

    // group pipeline together:
    let pipeline = vec![
        stage_filter_valid_years,
        stage_group_year,
        stage_sort_year_ascending,
        limit
    ];

    let mut results = movies.aggregate(pipeline, None).await?;

    while let Some(result) = results.next().await {
            let doc: YearSummary = bson::from_document(result?)?;
            println!("* // {:?} // {:?} {:?} //", doc._id, doc.movie_count, doc.movie_titles);

    }
    Ok(())



}