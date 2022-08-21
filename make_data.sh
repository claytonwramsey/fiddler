#!bash

# Script for generating openings using the Cute Chess command line interface.
# Runs an enormous number of hyper-bullet games.
# Requires the following environment variables:
# * `$RANDOM`: random seed for cute chess
# * `$BOOK`: path to PGN openings file

cutechess-cli \
    -srand $RANDOM \
    -pgnout games.pgn \
    -repeat \
    -recover \
    -tournament gauntlet \
    -rounds 500000 \
    -concurrency 16 \
    -ratinginterval 50 \
    -draw movenumber=50 movecount=5 score=20 \
    -openings file=$BOOK format=pgn order=random \
    -engine cmd=./target/release/fiddler_engine name=fiddler1 tc=40/2+0.05 \
    -engine cmd=./target/release/fiddler_engine name=fiddler2 tc=40/2+0.05 \
    -each timemargin=60000 option.Hash=512 proto=uci