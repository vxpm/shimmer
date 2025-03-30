# shimmer

a veeery WIP psx emulator. currently not intended for usage by anyone other than me.

components completion state (rough approximate):

| **Component** | **Status**  |
| ------------- | ----------- |
| CPU           | ~100%       |
| GPU           | ~75%        |
| DMA           | ~50%        |
| Timers        | ~25%        |
| GTE           | Not started |
| SPU           | Not started |
| MDEC          | Not started |
| CDROM         | ~25%        |
| SIO0          | ~50%        |
| SIO1          | Not planned |

# game compatibility list

here's a list with some games i test often.

- boots: goes past the playstation logo screen
- menu: gets to some kind of menu screen
- in game: possible to start playing the game, but doesn't go very far before problems arise
- playable: able to play the game without many (or any) problems

| **Game**         | **Compatibility** |
| ---------------- | ----------------- |
| Megaman X4       | Playable          |
| Castlevania SoTN | In game           |
| Megaman X6       | Menu              |
| Crash Bandicoot  | Menu              |
| Mortal Kombat II | Playable          |
| Alundra          | In game           |
| Worms            | Boots             |

# building

currently not possible if you're not me, as i'm using some libraries i made which aren't public yet.
