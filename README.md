## Heatmiser neohub "v3" (websocket/token) client

A (currently) low-level wrapper for the neoHub websocket API.

Upstream docs are available, with a free account, from https://dev.heatmiser.com/.
They are not very complete or accurate.

### Usage

Find your hub's address. Supposedly they respond to broadcast, but mine won't:
```bash
echo -n "hubseek" | nc -b -u 255.255.255.255 19790
```

I pulled the IP from my router's dashboard. Let's say the IP is `192.168.13.37`.

Next, create a token in the mobile app. The name doesn't matter. It looks like a `uuidv4`.

Export both of these as environment variables:

```bash
export NEOHUB_URL=wss://192.168.13.37:4243
export NEOHUB_TOKEN=69696969-6969-4969-6969-696969696969
```

Then, you can use the library:
```rust
let mut client = Client::from_env()?;
let result: Value = client.command_void(commands::GET_LIVE_DATA).await?;
println!("{}", result.to_string()));
```

Or one of the examples:
```bash
cargo run --example neohub-cli
```

```
>> GET_LIVE_DATA
{
  "CLOSE_DELAY": 0,
  "COOL_INPUT": false,
  "HOLIDAY_END": 0,
  "HUB_AWAY": false,
  "HUB_HOLIDAY": false,
```


### Examples

 * `dump-live-data` continually exports the GET_LIVE_DATA to `zstd`'d `jsonlines` files.


### Contributing / Future

I don't think the API is stable enough to provide structs, like the (generated)
`LiveData` struct, so I don't think adding those to the crate's API is a good idea.

Github issues or PRs, please.


### License

MIT OR Apache-2.0
