#![allow(clippy::too_many_arguments)]

use codec::{Decode, Encode};
#[cfg(feature = "collector")]
use hex::ToHex;
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
        prelude::AttachmentType,
    },
    prelude::*,
};
use sp_core::{
    crypto::{AccountId32, Ss58AddressFormat, Ss58Codec},
    Pair,
};
use std::borrow::Cow;
#[cfg(feature = "collector")]
use std::collections::HashMap;
#[cfg(feature = "funder")]
use subxt::{tx::PairSigner, OnlineClient, PolkadotConfig};

struct Handler;

#[cfg(feature = "funder")]
#[subxt::subxt(runtime_metadata_url = "wss://brainstorm.invarch.network:443")]
pub mod chain {}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content: Result<(String, Option<Vec<u8>>), String> = match command
                .data
                .name
                .as_str()
            {
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
                        if let Ok(account_id) = sp_core::crypto::AccountId32::from_ss58check(string)
                        {
                            let data = ctx.data.write().await;
                            if let Some((db, _, _)) = data.get::<DbData>() {
                                let discord_id = command.user.id.to_string().encode();
                                let discord_username = command.user.name.clone();

                                if let Ok(_) =
                                    db.insert(discord_id, (discord_username, account_id).encode())
                                {
                                    Ok((String::from("Account registered successfully!"), None))
                                } else {
                                    Ok((String::from("Error registering account."), None))
                                }
                            } else {
                                Ok((String::from("Error opening address database."), None))
                            }
                        } else {
                            Ok((String::from("Invalid address provided."), None))
                        }
                    } else {
                        Ok((String::from("No address provided."), None))
                    }
                }

                #[cfg(feature = "collector")]
                "generate_csv" => {
                    let data = ctx.data.write().await;
                    if let Some((db, guild_id, admin_role_id)) = data.get::<DbData>() {
                        if let Ok(true) = command
                            .user
                            .has_role(ctx.http.clone(), *guild_id, *admin_role_id)
                            .await
                        {
                            let mut wtr = csv::Writer::from_writer(vec![]);

                            db.into_iter().for_each(|res| {
                                if let Ok((key, value)) = res {
                                    if let (Ok(discord_id), Ok((discord_username, account_id))) = (
                                        String::decode(&mut key.to_vec().as_slice()),
                                        <(String, AccountId32)>::decode(
                                            &mut value.to_vec().as_slice(),
                                        ),
                                    ) {
                                        wtr.write_record(&[
                                            discord_id,
                                            discord_username,
                                            account_id.to_ss58check_with_version(
                                                Ss58AddressFormat::custom(2),
                                            ),
                                        ])
                                        .unwrap();
                                    };
                                }
                            });

                            if let Ok(sheet) = wtr.into_inner() {
                                Ok((String::from("addresses.csv"), Some(sheet)))
                            } else {
                                Ok((String::from("Failed to write csv."), None))
                            }
                        } else {
                            Ok((
                                String::from("This command is onlyavailable to admins."),
                                None,
                            ))
                        }
                    } else {
                        Ok((String::from("Error opening address database."), None))
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

                _ => Ok((String::from("Not implemented."), None)),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            let m = message
                                .content(match content.clone() {
                                    Ok((msg, _)) => msg,
                                    Err(post_info) => post_info,
                                })
                                .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);

                            if let Ok((filename, Some(file_data))) = content.clone() {
                                m.add_file(AttachmentType::Bytes {
                                    data: Cow::Owned(file_data),
                                    filename,
                                });
                            }

                            m
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
                .name("generate_csv")
                .description("Generate CSV of the registered addresses")
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

#[cfg(feature = "funder")]
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
    type Value = (sled::Db, u64, u64);
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

    let mut client = Client::builder(token, GatewayIntents::default())
        .event_handler(Handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    #[cfg(feature = "collector")]
    client.data.write().await.insert::<DbData>((
        sled::open("wallet_db").unwrap(),
        dotenv::var("server_id").unwrap().parse::<u64>().unwrap(),
        dotenv::var("admin_role_id")
            .unwrap()
            .parse::<u64>()
            .unwrap(),
    ));

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
