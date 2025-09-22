import sys
from subprocess import Popen, PIPE
from threading import Thread

# Make sure this is the location of the server file on the end user's device
SERVER_EXE_PATH = "./target/debug/gvsu-cis350-sporks"

with Popen([SERVER_EXE_PATH], stdin=PIPE, stdout=PIPE, stderr=PIPE, text=True) as process:
    print("opening server")
    while process.poll() is None: # while the process is running
        outputs, errors = process.communicate("--exit") # write "--exit" to stdin
        print("out: ", outputs)
        print("err: ", errors)
    print("process closed")
