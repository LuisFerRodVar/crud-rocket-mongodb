

#[macro_use] extern crate rocket;

use mongodb::bson::oid::ObjectId;
use rocket::serde::{json::Json, Deserialize, Serialize};
use mongodb::{bson::doc, Client, Collection};
use mongodb::results::InsertOneResult;
use std::env;
use dotenv::dotenv;
use rocket::State;
use futures::stream::TryStreamExt;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
    description: String,
}

struct MongoDB {
    collection: Collection<Item>,
}

#[post("/items", data = "<item>")]
async fn create_item(item: Json<Item>, mongo: &State<MongoDB>) -> Json<InsertOneResult> {
    let new_item = Item {
        id: None,
        name: item.name.clone(),
        description: item.description.clone(),
    };

    let result = mongo.collection.insert_one(new_item, None).await.unwrap();
    Json(result)
}

#[get("/items")]
async fn read_items(mongo: &State<MongoDB>) -> Json<Vec<Item>> {
    let cursor = mongo.collection.find(None, None).await.unwrap();
    let items: Vec<Item> = cursor.try_collect().await.unwrap();
    Json(items)
}

#[put("/items/<id>", data = "<item>")]
async fn update_item(id: &str, item: Json<Item>, mongo: &State<MongoDB>) -> Result<Json<Option<Item>>, String> {
    match ObjectId::parse_str(id) {
        Ok(object_id) => {
            let filter = doc! { "_id": object_id };
            let update = doc! {
                "$set": {
                    "name": &item.name,
                    "description": &item.description,
                }
            };

            match mongo.collection.update_one(filter.clone(), update, None).await {
                Ok(_) => {
                    match mongo.collection.find_one(filter, None).await {
                        Ok(updated_item) => Ok(Json(updated_item)),
                        Err(e) => Err(format!("Failed to fetch updated item: {}", e)),
                    }
                }
                Err(e) => Err(format!("Failed to update item: {}", e)),
            }
        }
        Err(_) => Err("Invalid ObjectId".to_string()),
    }
}

#[delete("/items/<id>")]
async fn delete_item(id: &str, mongo: &State<MongoDB>) -> Result<Json<bool>, String> {
    match ObjectId::parse_str(id) {
        Ok(object_id) => {
            let filter = doc! { "_id": object_id };
            match mongo.collection.delete_one(filter, None).await {
                Ok(result) => Ok(Json(result.deleted_count == 1)),
                Err(e) => Err(format!("Failed to delete item: {}", e)),
            }
        }
        Err(_) => Err("Invalid ObjectId".to_string()),
    }
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    dotenv().ok();
    let mongodb_uri = env::var("MONGODB_URI").expect("MONGODB_URI must be set");
    let client = Client::with_uri_str(&mongodb_uri).await.unwrap();
    let db = client.database("test");
    let collection = db.collection::<Item>("items");

    rocket::build()
        .manage(MongoDB { collection })
        .mount("/", routes![create_item, read_items, update_item, delete_item])
        .launch()
        .await?;

    Ok(())
}

