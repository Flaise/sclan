SCLAN
--------------------------------------------
Simple Communication over Local Area Network

Easily send messages - text, notes, links, whatever - from one of your computers to another or talk to co-workers on an office network. Press [Tab] to pick which computer to send the message to, press [Enter] to start typing, then press [Enter] to send. That's all.

![screenshot](/screenshots/sclan_01.png)

It's easy to install, uses very little memory and CPU power, and runs on just about every computer that has a terminal - meaning it should work on almost every desktop machine and laptop. It automatically locates other computers running sclan on the same network. If you plug your computers into the same router (or set them up on the same WiFi access point) it should just work on its own after a few seconds.

Press up and down to select a message in order to copy it to the system clipboard for a convenient way to move the text to a different program on your computer. Copy and paste in sclan are [Alt+C] and [Alt+V] because pressing [Ctrl+C] in a terminal is the standard way to end a program on all platforms.

Whenever there's a sclan.log file in the current working directory, incoming and outgoing messages will be logged to that file. Press [L] to create the file and start logging if there isn't one.


Limitations
-----------
* Guest networks are usually configured so that computers cannot scan the router to see what else is connected. Currently, sclan does not have a way to function on this kind of network and you will instead want to use an instant messenger program powered by a central server.

* Although the end-to-end encryption prevents your messages from being intercepted by a third party while you already have a connection to the other machine, sclan does not have a way to verify that other machines on the network are who they say they are. Whatever name the other user has picked for their computer is the name that will show up in the network list in sclan. This is one of the reasons why guest networks usually don't allow themselves to be scanned. Please don't use sclan to move an important password to another computer unless you know for sure what's on your network. Also be aware that all data upon arrival will be displayed in plain text, not obscured in any way, making you vulnerable to screen-reading malware and someone looking over your shoulder.

* It won't locate other computers via IPv6 connections. If you're using a router that does not support IPv4 (some newer routers) then sclan won't work yet.


Planned Features
----------------
* File transfer.
* IPv6 support.


Install and Run on Windows
--------------------------
Go to the [Releases](https://github.com/Flaise/sclan/releases) page on the Github repository, download the executable, and double-click it to run it. It doesn't require any particular installation. Just put it somewhere on your computer and it will run. Or, to install and run from the command line, install with cargo (below).

The controls are always displayed in the lower left corner.

All Other Installations (Advanced Users)
----------------------------------------
For the time being, my friends using Unix systems will have to build from source. Advanced Windows users can also do this if desired.

Install the Rust toolchain if you haven't already. The easiest way is by following the directions on the [Rustup](https://rustup.rs/) website. Then open a terminal. (On Windows this can be done by pressing the windows key and typing "cmd" and pressing enter.) Use the command:

    cargo install sclan
    
Then, to run sclan, type

    sclan
    
Alternatively you can use git to clone this repository and from inside your cloned copy of the repository, use

    cargo run


License
---------------------
The Clear BSD License

See LICENSE.txt in this directory for the details.

