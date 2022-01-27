"""
A simple script which will create all the lines in square.rs.
"""

FILE_NAMES = "ABCDEFGH"
RANK_NAMES = [str(i+1) for i in range(8)]

for i in range(64):
    #A1, then B1, then (...) A2, B2 (...) G8, H8
    file_index = i % 8
    rank_index = i // 8
    square_name = FILE_NAMES[file_index] + RANK_NAMES[rank_index]
    print("#[allow(dead_code)]")
    line = f"pub const {square_name}: Square = Square({i});"
    print(line)