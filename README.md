# Line Server

In this exercise, you will build and document a system that is capable of serving lines out of a file to network clients.

## Specification

Your system should act as a network server that serves individual lines of an immutable text file over the network to clients using the following protocol:

**GET N**

If `N` is a valid line number for the given text file, return "OK\r\n" and then the nnnn-th line of the specified text file.

If `N` is not a valid line number for the given text file, return "ERR\r\n". The first line of the file is line 1 (not line 0).

**QUIT**

Disconnect client

**SHUTDOWN**

Shutdown the server

The server should listen for TCP connections on port 7878.

Your server must support multiple simultaneous clients at a time. The system should perform well for small and large files. The system should perform well as the number of GET requests per unit time increases.

You may pre-process the text file in any way that you wish so long as the server behaves correctly.

Server activity should be observable, both live and after closing the program, allowing auditing of activity that happened on the server at any time. There should be sufficient data to draw conclusions about connection lengths, requests made, and any other typical activity insights.

The text file will have the following properties:
* Each line is terminated with a newline ("\n").
* Any given line will fit into memory.
* The line is valid ASCII (e.g. not Unicode).

### Example

Suppose the given file is:
```
the
quick brown
fox jumps over the
lazy dog
```

Then you could imagine the following transcript with your server:

```
Client => GET 1
Server <= OK
Server <= the
Client => GET -3
Server <= ERR
Client => GET 4
Server <= OK
Server <= lazy dog
Client => QUIT
<<Server disconnects from client 1 >>
Client => SHUTDOWN
<<Server disconnects from ALL clients >>
```

## Running the line server

The line server executable expects two arguments - the first is the TCP endpoint to use for the server and the second is a filename that is used to send to clients.

### Example

Suppose you want to serve the `foo.txt` file in this repository to other clients, and you want to handle any IPV4 address on port 7878. Run the program as follows.

`cargo run -- "0.0.0.0:7878" "foo.txt"`
