<p align="center">
<img width="1774" height="887" alt="exec-bf36c6e8-3272-4735-81de-307dd84ec1a4" src="https://github.com/user-attachments/assets/0e9653fb-9e3c-447e-a970-ee92da5b8523" />
</p>

## astrolight - open source for lucky65 v1 keyboard
Why is this exist in the first place? It's pretty easy since we already have the vendor's software, I just have plenty of tokens and thought I would like to feed it into ChatGPT to do the rest for me lol. You just can use the vendor's software but for me it was have plenty of disadvantage that's I don't really like, for example: 
  * Electron: There no problem with it but you will have problem when the RAM start eating up by other applications.
  * Outdated UI: Its just me or the UI look so outdated and "Chinese" made (no offense, I love China).
  * Missing QOL Features: Everytime you booted it up, it gonna ask you which keyboard are you using? It would be handy if it can remember what I selected in the first place.
  * Limited Lighting Options: Can't blame them because it is minium effort since most of the Chinese keyboard doesn't care much about customization for LED beside "Some" classic lighting effect.

What does "astrolight" offered to you, in order to replace the original vendor firmware?
  * Open source: Yeah I don't wanted to make it closed source, it is an outdated keyboard with no reason for me to make it closed.
  * Fully adapted the original software features: Lighting, Macros, Keyboard Remap, Profiles
  * Optimized backend: with the current stack, I'm using Tauri and Rust and multi-thread managed to make the app run effective as possible
  * QOL: Adding lighting template sharing, keyboard settings export, Games/Apps detection for lighting
  * Bugs: Yeah it's vibecoded, expect bugs
