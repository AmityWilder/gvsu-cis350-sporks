import sys
from subprocess import Popen, PIPE
from threading import Thread

SERVER_EXE_PATH = "./target/debug/gvsu-cis350-sporks" # CHANGE THIS

with Popen([SERVER_EXE_PATH], stdin=PIPE, stdout=PIPE, stderr=PIPE, text=True) as proc:
    print("opening server")
    while proc.poll() is None:
        outs, errs = proc.communicate("--exit")
        for out in outs:
            print("out: ", out)
        for err in errs:
            print("err: ", errs)
    print("process closed")
