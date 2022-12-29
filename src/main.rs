#![allow(clippy::too_many_arguments)]

#[cfg(feature = "collector")]
use hex::ToHex;
#[cfg(feature = "collector")]
use rusoto_core::Region;
#[cfg(feature = "collector")]
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
use sp_core::{
    crypto::{AccountId32, Ss58Codec},
    Pair,
};
#[cfg(feature = "collector")]
use std::collections::HashMap;
use subxt::{tx::PairSigner, OnlineClient, PolkadotConfig};

struct Handler;

#[cfg(feature = "funder")]
#[subxt::subxt(runtime_metadata_url = "wss://brainstorm.invarch.network:443")]
pub mod chain {}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content: Result<String, String> = match command.data.name.as_str() {
                #[cfg(feature = "collector")]
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
                                    rusoto_core::RusotoError::Service(
                                        rusoto_dynamodb::PutItemError::ConditionalCheckFailed(_),
                                    ) => Ok(String::from("Your account is already in the list.")),

                                    _ => Ok(String::from("An error has occurred")),
                                }
                            } else {
                                Ok(String::from("Address added succesfully!"))
                            }
                        } else {
                            Ok(String::from("Please provide a valid address."))
                        }
                    } else {
                        Ok(String::from("Please provide a valid address."))
                    }
                }

                #[cfg(feature = "collector")]
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
                                    rusoto_core::RusotoError::Service(
                                        rusoto_dynamodb::PutItemError::ConditionalCheckFailed(_),
                                    ) => Ok(String::from("Your account is not in the list.")),

                                    _ => Ok(String::from("An error has occurred")),
                                }
                            } else {
                                Ok(String::from("Address added succesfully!"))
                            }
                        } else {
                            Ok(String::from("Please provide a valid address."))
                        }
                    } else {
                        Ok(String::from("Please provide a valid address."))
                    }
                }

                #[cfg(feature = "funder")]
                "fund_wallet" => {
                    if let Some(o) = command.data.options.get(0) {
                        if let Some(options) = o.resolved.as_ref() {
                            if let ApplicationCommandInteractionDataOptionValue::String(string) =
                                options
                            {
                                if let Ok(accountid) =
                                    sp_core::crypto::AccountId32::from_ss58check(string)
                                {
                                    let data = ctx.data.write().await;
                                    if let Some((api, pair_signer, amount, db)) =
                                        data.get::<DbData>()
                                    {
                                        match handle_funding_command(
                                            api,
                                            pair_signer,
                                            *amount,
                                            db,
                                            command.user.id.to_string(),
                                            accountid,
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                Ok(String::from("Funding call submitted on chain!"))
                                            }
                                            Err(e) => Err(e),
                                        }
                                    } else {
                                        Err(String::from("Error trying to fund account."))
                                    }
                                } else {
                                    Ok(String::from("Please provide a valid address."))
                                }
                            } else {
                                Ok(String::from("Please provide a valid address."))
                            }
                        } else {
                            Err(String::from("Error trying to fund account."))
                        }
                    } else {
                        Err(String::from("Error trying to fund account."))
                    }
                }

                _ => Ok(String::from("Not implemented.")),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .content(match content.clone() {
                                    Ok(msg) => msg,
                                    Err(post_info) => post_info,
                                })
                                .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                        })
                })
                .await
            {
                println!("Cannot respond to slash command: {}", why);
            }

            //  if let Err(why) = content {
            //      panic!("Called panic on a restart-required error: '{}'", why);
            //  }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        #[cfg(feature = "collector")]
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

        #[cfg(feature = "collector")]
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

        #[cfg(feature = "funder")]
        ApplicationCommand::create_global_application_command(&ctx.http, |command| {
            command
                .name("fund_wallet")
                .description("Fund your wallet with 🧠⛈️ test tokens!")
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

async fn handle_funding_command(
    api: &OnlineClient<PolkadotConfig>,
    pair_signer: &PairSigner<PolkadotConfig, sp_core::sr25519::Pair>,
    amount: u128,
    db: &sled::Db,
    discord_id: String,
    account_id: AccountId32,
) -> Result<(), String> {
    if !db
        .contains_key(discord_id.clone())
        .map_err(|_| String::from("Error trying to fund account."))?
    {
        let transfer_call = chain::tx()
            .balances()
            .transfer(account_id.clone().into(), amount);

        api.tx()
            .sign_and_submit_default(&transfer_call, pair_signer)
            .await
            .map_err(|_| String::from("Error trying to fund account."))?;

        db.insert(discord_id, account_id.to_string().as_bytes())
            .map_err(|_| String::from("Error trying to fund account."))?;

        Ok(())
    } else {
        Err(String::from("You already registered in the faucet."))
    }
}

struct DbData;

impl TypeMapKey for DbData {
    #[cfg(feature = "collector")]
    type Value = DynamoDbClient;
    #[cfg(feature = "funder")]
    type Value = (
        OnlineClient<PolkadotConfig>,
        PairSigner<PolkadotConfig, sp_core::sr25519::Pair>,
        u128,
        sled::Db,
    );
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let token = dotenv::var("token").unwrap();

    let application_id: u64 = dotenv::var("app_id").unwrap().parse().unwrap();

    let mut client = Client::builder(token)
        .event_handler(Handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    #[cfg(feature = "collector")]
    {
        let db_client = DynamoDbClient::new(Region::UsEast1);
        let mut data = client.data.write().await;
        data.insert::<DbData>(db_client);
    }

    #[cfg(feature = "funder")]
    client.data.write().await.insert::<DbData>((
        OnlineClient::<PolkadotConfig>::from_url("wss://brainstorm.invarch.network:443")
            .await
            .unwrap(),
        PairSigner::new(
            sp_core::sr25519::Pair::from_phrase(dotenv::var("signer").unwrap().as_str(), None)
                .unwrap()
                .0,
        ),
        dotenv::var("amount").unwrap().parse::<u128>().unwrap() * 1_000_000_000_000,
        sled::open("user_db").unwrap(),
    ));

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
