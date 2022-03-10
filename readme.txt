SCLAN
--------------------------------------------
Simple Communication over Local Area Network

Easily send messages - text, notes, links, whatever - from one of your computers to another or talk to co-workers on an office network. Press [Tab] to pick which computer to send the message to, press [Enter] to start typing, then press [Enter] to send. That's all.

Easy to install, uses very little memory and CPU power, and runs on just about every computer that has a terminal - meaning it should work on just about every desktop machine and laptop. It automatically locates other computers running sclan on the same network. If you plug your computers into the same router (or set them up on the same WiFi access point) it should just work on its own after a few seconds.

Press up and down to select a message in order to copy it to the system clipboard for a convenient way to move the text to a different program on your computer. Copy and paste in sclan are [Alt+C] and [Alt+V] because pressing [Ctrl+C] in a terminal is the standard way to end a program on all platforms.


Limitations
-----------
* Guest networks are usually configured so that computers cannot scan the router to see what else is connected. Currently, sclan does not have a way to function on this kind of network and you will instead want to use an instant messenger program powered by a central server.

* Although the end-to-end encryption prevents your messages from being intercepted by a third party while you already have a connection to the other machine, sclan does not have a way to verify that other machines on the network are who they say they are. Whatever name the other user has picked for their computer is the name that will show up in the network list in sclan. This is one of the reasons why guest networks usually don't allow themselves to be scanned. Please don't use sclan to move an important password to another computer unless you know for sure what's on your network. Also be aware that all data upon arrival will be displayed in plain text, not obscured in any way, making you vulnerable to screen-reading malware and someone looking over your shoulder.


Planned Features
----------------
* File transfer.
* Optional logging so you can find your links and other important messages even after restarting sclan.


Install and Run
---------------
    Windows:
Go to the Releases page on the Github repository, download the executable, and double-click it to run it. Or, for a command line interface, install with cargo (below). It doesn't require any particular installation. Just put it somewhere on your computer and it will run.
    
    All platforms:
Requires the Rust toolchain to already be installed. Open a terminal. (On Windows this can be done by pressing the windows key and typing "cmd" and pressing enter.) Use the command:
    cargo install sclan
Then, to run sclan, type
    sclan

The controls are always displayed in the lower left corner.


License
---------------------
The Clear BSD License

See LICENSE.txt in this directory for the details.

