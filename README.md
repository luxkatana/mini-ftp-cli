# A mini ftp cli using a custom protocol over sockets!


## What does this magic do? :sparkles:

This project is a mini-ftp application that can pull files from the server using a custom protocol. It's not as efficient as existing protocoles such as scp, ftp etc but I learned a lot because of this project!

*Expect some ~~bugs~~ \*cough\* features :smile:*

This project consists of a server and a client, and it's obviously that you need to run the server and the client to make the use of this project!

***Please make sure that you have Rust, Rust can be installed <a href='https://www.rust-lang.org/learn/get-started'> rust get started page </a>***

***Executables can be found under the releases tab on github***

## Building the project by source
### How I build client plz tell me

1. Change directory to ``/ftp-client``
2. Run ``cargo b -r`` to build the program in release mode, the final executable is ``/ftp-client/target/release/ftp-client`` or ``/ftp-client/target/release/ftp-client.exe`` 


### How I build server plz tell me   

1. Change directory to ``/ftp-server``
2. Run ``cargo b -r`` to build the program in release mode, the final executable is ``/ftp-server/target/release/ftp-server`` or ``/ftp-server/target/release/ftp-server.exe`` 

*NOTE: Pleaes make sure that port 13360 is open on the firewall!*


## Keys in ftp-client

I implemented the following keys when doing stuff in ftp-client:
- ``k`` and ``<UP_ARROW>`` keys to move up
- ``j`` and `` <DOWN_ARROW>`` keys to move down
- ``s`` to save a file/client from the server
- ``<LEFT_ARROW>`` To go back in time  :sparkles:
- ``<Enter> `` to select a file/folder and do something useful with it ig
- ``q`` and ``<KEY_ESCAPE>`` to escape from the ftp-client 



## What I learnt of this project

I learnt the following things:
1. Rust can be challenging, but it works
2. How TLS works in rust
3. How to make your own protocol



***By the way, /certificates consists of certificates in case you didn't know that..!***

