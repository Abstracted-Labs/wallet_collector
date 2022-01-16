use dotenv;
use hex::ToHex;
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient};
use serenity::{
    async_trait,
    model::{
        gateway::Ready,
        interactions::{
            application_command::{
                ApplicationCommand, ApplicationCommandInteractionDataOptionValue,
                ApplicationCommandOptionType,
            },
            Interaction, InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
        },
    },
    prelude::*,
};
use sp_core::crypto::Ss58Codec;
use std::collections::HashMap;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "give_wallet" => {
                    let options = command
                        .data
                        .options
                        .get(0)
                        .expect("Expected user option")
                        .resolved
                        .as_ref()
                        .expect("Expected user object");

                    if let ApplicationCommandInteractionDataOptionValue::String(string) = options {
                        if let Ok(accountid) = sp_core::crypto::AccountId32::from_ss58check(string)
                        {
                            let db_client = ctx.data.write().await;
                            let db_client = db_client.get::<DbData>().unwrap();

                            let result = db_client
                                .put_item(rusoto_dynamodb::PutItemInput {
                                    condition_expression: Some(String::from(
                                        "attribute_not_exists",
                                    )),
                                    conditional_operator: None,
                                    expected: None,
                                    expression_attribute_names: None,
                                    expression_attribute_values: None,
                                    item: HashMap::from_iter(
                                        vec![(
                                            command.user.id.to_string(),
                                            AttributeValue {
                                                b: None,
                                                bool: None,
                                                bs: None,
                                                l: None,
                                                m: None,
                                                n: None,
                                                ns: None,
                                                null: None,
                                                s: Some(accountid.encode_hex::<String>()),
                                                ss: None,
                                            },
                                        )]
                                        .into_iter(),
                                    ),
                                    return_consumed_capacity: None,
                                    return_item_collection_metrics: None,
                                    return_values: None,
                                    table_name: String::from("airdrops"),
                                })
                                .await;

                            if let Err(error) = result {
                                println!("Error: {:?}", error);
                                match error {
                                    rusoto_core::RusotoError::Service(s) => match s {
                                        rusoto_dynamodb::PutItemError::ConditionalCheckFailed(
                                            _,
                                        ) => String::from("Your account is already in the list."),
                                        _ => String::from("An error has occurred"),
                                    },
                                    _ => String::from("An error has occurred"),
                                }
                            } else {
                                String::from("Address added succesfully!")
                            }
                        } else {
                            String::from("Please provide a valid address.")
                        }
                    } else {
                        String::from("Please provide a valid address.")
                    }
                }

                "replace_wallet" => {
                    let options = command
                        .data
                        .options
                        .get(0)
                        .expect("Expected user option")
                        .resolved
                        .as_ref()
                        .expect("Expected user object");

                    if let ApplicationCommandInteractionDataOptionValue::String(string) = options {
                        if let Ok(accountid) = sp_core::crypto::AccountId32::from_ss58check(string)
                        {
                            let db_client = ctx.data.write().await;
                            let db_client = db_client.get::<DbData>().unwrap();

                            let result = db_client
                                .put_item(rusoto_dynamodb::PutItemInput {
                                    condition_expression: Some(String::from("attribute_exists")),
                                    conditional_operator: None,
                                    expected: None,
                                    expression_attribute_names: None,
                                    expression_attribute_values: None,
                                    item: HashMap::from_iter(
                                        vec![(
                                            command.user.id.to_string(),
                                            AttributeValue {
                                                b: None,
                                                bool: None,
                                                bs: None,
                                                l: None,
                                                m: None,
                                                n: None,
                                                ns: None,
                                                null: None,
                                                s: Some(accountid.encode_hex::<String>()),
                                                ss: None,
                                            },
                                        )]
                                        .into_iter(),
                                    ),
                                    return_consumed_capacity: None,
                                    return_item_collection_metrics: None,
                                    return_values: None,
                                    table_name: String::from("airdrops"),
                                })
                                .await;

                            if let Err(error) = result {
                                println!("Error: {:?}", error);
                                match error {
                                    rusoto_core::RusotoError::Service(s) => match s {
                                        rusoto_dynamodb::PutItemError::ConditionalCheckFailed(
                                            _,
                                        ) => String::from("Your account is not in the list."),
                                        _ => String::from("An error has occurred"),
                                    },
                                    _ => String::from("An error has occurred"),
                                }
                            } else {
                                String::from("Address added succesfully!")
                            }
                        } else {
                            String::from("Please provide a valid address.")
                        }
                    } else {
                        String::from("Please provide a valid address.")
                    }
                }

                _ => String::from("Not implemented."),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .content(content)
                                .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                        })
                })
                .await
            {
                println!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        ApplicationCommand::create_global_application_command(&ctx.http, |command| {
            command
                .name("give_wallet")
                .description("Give the bot your wallet")
                .create_option(|option| {
                    option
                        .name("wallet")
                        .description("The actual wallet")
                        .kind(ApplicationCommandOptionType::String)
                        .required(true)
                })
        })
        .await
        .unwrap();

        ApplicationCommand::create_global_application_command(&ctx.http, |command| {
            command
                .name("replace_wallet")
                .description("Replace the wallet you gave to the bot")
                .create_option(|option| {
                    option
                        .name("wallet")
                        .description("The actual wallet")
                        .kind(ApplicationCommandOptionType::String)
                        .required(true)
                })
        })
        .await
        .unwrap();
    }
}

struct DbData;

impl TypeMapKey for DbData {
    type Value = DynamoDbClient;
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let token = dotenv::var("token").unwrap();

    let application_id: u64 = dotenv::var("app_id").unwrap();

    let mut client = Client::builder(token)
        .event_handler(Handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");
    {
        let db_client = DynamoDbClient::new(Region::UsEast1);
        let mut data = client.data.write().await;
        data.insert::<DbData>(db_client);
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
