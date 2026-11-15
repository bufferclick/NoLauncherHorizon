Fabric n Horizon v@VERSION@
===========================

Your Horizon instances, one page.

Double-click FH.exe. Your browser opens on a page — JLac's window, with your Horizon instances
down the left and the one you pick on the right. Press Launch: if it has to, it signs you in with
Microsoft (the code is on the page, on your clipboard, and as a QR code for a phone), installs the
build the instance plays when this computer has none, takes the game's files from your own
Minecraft the way the launcher does, and starts the game. The tab says how far it got, and the
page stays open behind the game: close the tab whenever you like, and open it again from the
address in the log.

It lists exactly the instances the Horizon launcher has — the same folder, the same worlds and
options, the same sign-in — so FH and the launcher can live side by side. Instances that play
Minecraft: Java Edition are listed too, with their Launch button off: FH starts Horizon
instances, and Minecraft ones are next.

Signing in needs no account of yours beyond Microsoft's: there is no app to register and nothing
to configure.

Where things are (Windows):

  %APPDATA%\horizon-launcher\fh\fh.log                what happened, line by line

  %APPDATA%\horizon-launcher\instances.json           the instances (shared with the launcher)
  %APPDATA%\horizon-launcher\horizon                  your worlds and options

If a launch does not start, the page says why and the log says more.

Options: FH.exe --version --help

MIT licence. Not an official Minecraft product; not approved by or associated with Mojang or
Microsoft. Horizon is made by the Horizon Team; Fabric n Horizon is Buffer's page over it, styled
after JLac's window.
