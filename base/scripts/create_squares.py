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
A simple script which will create all the lines in square.rs.
"""

FILE_NAMES = "ABCDEFGH"
RANK_NAMES = [str(i+1) for i in range(8)]
print("pub enum Square {")

for i in range(64):
    #A1, then B1, then (...) A2, B2 (...) G8, H8
    file_index = i % 8
    rank_index = i // 8
    square_name = FILE_NAMES[file_index] + RANK_NAMES[rank_index]
    line = f"\t{square_name} = {i},"
    print(line)

print("}")