# tts-proxy

A simple proxy server for the vo.codes API necessitated by their CORS policy.

## Usage

Normally, the proxy should be started through the GUI, which takes care of its configuration. However, should one require a manual setup, the following steps should be taken:

1. Build the binary in release mode:

```bash
$ cargo build --release
$ ./target/release/obs-tts-proxy --help
tts-proxy 0.1.0
Start the TTS proxy with the given configuration

USAGE:
    obs-tts-proxy [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -r, --retry-attempts <ATTEMPTS>    The number of retry attempts the proxy should make if the API
                                       fails to respond [default: 3]
    -l, --log-directory <PATH>         The path to the directory to store the logs in.
    -p, --port <PORT>                  The port to run the proxy on [default: 3031]
    -t, --timeout <SECONDS>            The maximum number of seconds a request to the API may take
                                       [default: 180]
    -a, --api-url <URL>                The API URL to proxy requests to [default:
```

2. Start the proxy with the desired flags, optionally enabling logging. Note that specifying a log directory will disable STDERR logging.

```bash
$ RUST_LOG=info ./target/release/obs-tts-proxy -p 7777 -r 1
Jul 07 15:07:40.044  INFO Server::run{addr=127.0.0.1:7777}: warp::server: listening on http://127.0.0.1:7777
```
