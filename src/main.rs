use scythe_take_home::LineServer;
use std::env;
use std::io::{Error, ErrorKind, Result};
use std::sync::Arc;

mod scythe_take_home {
    use std::{
        io::{self, Error, Result},
        str::FromStr,
        sync::Arc,
    };

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::{
        net::{TcpListener, TcpStream},
        sync::Mutex,
    };

    use tokio::fs::File;
    use tokio_util::{sync::CancellationToken, task::TaskTracker};

    #[derive(PartialEq)]
    enum TcpRequest {
        Get,
        Shutdown,
        Quit,
    }

    pub struct LineServer {
        endpoint: String,
        filename: String,
    }

    impl FromStr for TcpRequest {
        type Err = ();

        fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
            match s {
                s if s.starts_with("GET") => Ok(TcpRequest::Get),
                "SHUTDOWN" => Ok(TcpRequest::Shutdown),
                "QUIT" => Ok(TcpRequest::Quit),
                _ => Err(()),
            }
        }
    }

    impl LineServer {
        // Creates a new instance of a LineServer
        pub fn new(endpoint: &str, filename: &str) -> LineServer {
            LineServer {
                endpoint: endpoint.to_string(),
                filename: filename.to_string(),
            }
        }

        // The function the runs the main loop that takes care of accepting incoming connections
        // and gracefully shutting down when a `TcpRequest::Shutdown` is received by a client.
        pub async fn run(self: Arc<Self>) -> Result<()> {
            let token = CancellationToken::new();
            let listener = TcpListener::bind(&self.endpoint).await?;
            let tracker = TaskTracker::new();

            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        println!("Server has been shutdown!");
                        tracker.close();
                        break;
                    }
                    Ok((stream, _)) = listener.accept() => {
                        println!("Connection established!");
                        let shared_stream = Arc::new(Mutex::new(stream));
                        let cloned_self = self.clone();

                        // Each task thats spawned should pay attention to the cancellation token.
                        // Lets clone it and shutdown the stream when the token is cancelled by
                        // another task.
                        let cloned_token = token.clone();

                        tracker.spawn(async move {
                            tokio::select! {
                                _ = cloned_token.cancelled() => {
                                    let cloned_stream = Arc::clone(&shared_stream);
                                    let mut acquire_stream = cloned_stream.lock().await;
                                    let _ = acquire_stream.shutdown().await;
                                },
                                _ = cloned_self.handle_connection(Arc::clone(&shared_stream), &cloned_token) => ()
                            }
                        });
                    }
                }
            }

            println!("Waiting for all tasks to stop..");

            tracker.wait().await;

            println!("All tasks have stopped, have a nice day!");

            Ok(())
        }

        // This function takes care of handling a single client connection by parsing
        // through the `TcpRequest`s that are sent to the server.
        async fn handle_connection(
            &self,
            stream: Arc<Mutex<TcpStream>>,
            token: &CancellationToken,
        ) -> Result<()> {
            let mut acquire_stream = stream.lock().await;
            let (mut read_half, mut write_half) = tokio::io::split(&mut *acquire_stream);
            let reader = BufReader::new(&mut read_half);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await? {
                let trimmed_line = line.trim();

                if let Ok(request) = TcpRequest::from_str(trimmed_line) {
                    match request {
                        TcpRequest::Get => {
                            self.handle_get_request(&mut write_half, trimmed_line)
                                .await?
                        }
                        TcpRequest::Quit | TcpRequest::Shutdown => {
                            if request == TcpRequest::Shutdown {
                                // Let other tasks know that the server needs to shutdown
                                println!("Server is shutting down..");
                                token.cancel();
                            }
                            break;
                        }
                    }
                } else {
                    eprintln!("Invalid string found \'{line}\'");
                }
            }

            Ok(())
        }

        // This function handles a `GET` request as defined in the take home.
        // NTS: Should also be an async function if we don't want to block on reading
        // from a file and writing back to the client.
        async fn handle_get_request(
            &self,
            write_stream: &mut tokio::io::WriteHalf<&mut tokio::net::TcpStream>,
            get_str: &str,
        ) -> Result<()> {
            let no_get_str = get_str.replace("GET ", "");
            let parse_num = no_get_str.trim().parse::<usize>();
            let mut write_line = "ERR\n".to_string();

            // Check if we parsed the string as usize
            match parse_num {
                Ok(file_line_num) => {
                    // Read a single line from the file
                    match self.read_line_from_file(file_line_num).await {
                        Ok(line) => {
                            write_line = line + "\n";
                        }
                        Err(err) => {
                            eprintln!("Error while reading line number \'{file_line_num}\': {err}")
                        }
                    }
                }
                Err(err) => eprintln!("Error while parsing the string \'{no_get_str}\': {err}"),
            }

            // Write data back to the client
            if let Err(err) = write_stream.write_all(write_line.as_bytes()).await {
                eprintln!("Error while writing to the stream: {err}");
            }

            Ok(())
        }

        // This function asynchronously reads a specific file line number from the file `filename`.
        // NTS: This should be an async function to avoid blocking on reading from the file.
        async fn read_line_from_file(&self, line_number: usize) -> Result<String> {
            let file = File::open(&self.filename).await?;
            let reader = BufReader::new(file);
            let mut lines = reader.lines();
            let mut index = 0;

            while let Some(line) = lines.next_line().await? {
                // Return the line number (ex. 'GET 2' should return index 1)
                if index + 1 == line_number {
                    return Ok(line);
                }
                index += 1;
            }

            Err(Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "line number \'{line_number}\' is out of the range \'1 to {}\'",
                    index + 1
                ),
            ))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    const EXPECTED_ARGS_LEN: usize = 3;
    let args: Vec<String> = env::args().collect();
    let len = args.len();

    if args.len() != EXPECTED_ARGS_LEN {
        println!("Usage: line_server <endpoint> <filename>");
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Expected {EXPECTED_ARGS_LEN} arguments but got {len}"),
        ));
    }

    let endpoint = &args[1];
    let filename = &args[2];

    // Create a new instance of a LineServer
    let server = Arc::new(LineServer::new(endpoint, filename));

    // Run the LineServer
    server.run().await?;

    Ok(())
}
