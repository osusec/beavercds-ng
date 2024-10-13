#!/bin/sh

# no stderr
exec 2>/dev/null

# dir
cd /chal

# timeout after 20 sec
# @AUTHOR: make sure to set the propery entry point
#                             <---| don't touch anything left
#                                 | unless you need a longer timeout
timeout -k1 20 stdbuf -i0 -o0 -e0 ./notsh
#           ^^ 20 sec timeout