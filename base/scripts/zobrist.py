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
print(f"const BLACKTO_MOVE_KEY: u64 = {rand.getrandbits(64)};")