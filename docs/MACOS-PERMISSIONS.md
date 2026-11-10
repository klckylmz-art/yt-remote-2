# macOS permissions — experimental YT Remote 3.0

The teacher controls the STUDENT computer. The student must explicitly
allow each session. Only after that action does the native app check the
student's grants in this exact order: Screen Recording, Accessibility,
then Input Monitoring. Refusal fails closed;
the teacher cannot approve these OS grants remotely through this app.

On the student's Mac, open System Settings > Privacy & Security and allow
YT Remote 3.0 in Screen Recording (or Screen & System Audio Recording) and
Accessibility, then Input Monitoring. Restart the app if macOS requests it. A grant allows the OS
operation; it does not approve future YT Remote sessions automatically.
The app implements no audio capture despite macOS's combined settings name.
Input Monitoring is requested in this order at the product owner's explicit
request. The app installs no global keyboard listener/event tap or keystroke
recorder; ordinary remote input injection uses Accessibility. This additional
grant does not enable unattended control or replace student consent.

After granting permissions, use a fresh invitation and explicitly press
ALLOW SCREEN & KEYBOARD again. DENY or STOP SHARING NOW ends the session.

Prototype bundles are ad-hoc signed, not notarized. Follow normal macOS
user-controlled application approval; do not disable Gatekeeper or other
system protections. Check the included SOURCE.txt for the exact build.

Native minimize uses the NSView supplied by Minifb to obtain its NSWindow
and calls the public AppKit miniaturize operation on the main UI thread.
Physical permission, Retina, fullscreen and minimize tests are still
required on both Intel and Apple Silicon Macs.
