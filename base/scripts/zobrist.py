"""
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
"""

"""
This script generates the static constants in src/zobrist.rs, and prints the
resulting constants to the console.
"""

import random as rand

rand.seed(12345)

#Square keys
print("const SQUARE_KEYS: [[[u64; 2]; NUM_PIECE_TYPES]; 64] = [")
for i in range(64):
    #Piece type
    print("    [")
    for j in range(6):
        #Color
        print(f"        [{rand.getrandbits(64)}, {rand.getrandbits(64)}], ")
    print("    ],")
print("];")

#Castle keys
print("const CASTLE_KEYS: [u64; 4] = [")
for i in range(4):
    print(f"    {rand.getrandbits(64)},")
print("];")

#En passant keys
print("const EP_KEYS: [u64; 8] = [")
for i in range(8):
    print(f"    {rand.getrandbits(64)},")
print("];")

#Black to move key
print(f"const BLACK_TO_MOVE_KEY: u64 = {rand.getrandbits(64)};")