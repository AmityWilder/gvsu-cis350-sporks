import time, xmlrpc.client, subprocess, atexit, datetime, socket
from typing import Literal, Iterable
from datetime import datetime


def open_server(build: Literal["debug", "release"]):
    """Open a server on the requested build, listening at the requested socket. Use as options:
    build=DEBUG or RELEASE - The build directory to search for the executable in (starting in target)
    socket=SocketAddr - The socket to listen at.
    """
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        addr,port = s.getsockname()
    print(f"requesting server on address {addr}:{port}")
    # open the server in parallel
    srv = subprocess.Popen([f"./target/{build}/gvsu-cis350-sporks", f"{addr}:{port}"])
    # wait for the server to be open
    waited = 0
    proxy = xmlrpc.client.ServerProxy(f"http://{addr}:{port}", allow_none=True, use_datetime=True, use_builtin_types=True, verbose=True)
    print(f"wipe_slots: {proxy.wipe_slots}")
    # while True:
    #     try:
    #         proxy.wipe_slots({})
    #         break
    #     except xmlrpc.client.Fault as fault:
    #         if fault.faultCode == 404:
    #             time.sleep(0.01)
    #             waited += 0.01
    #             if waited >= 1:
    #                 raise fault
    #         else:
    #             raise fault


    def close_server():
        print("attempting to close server")
        if srv.poll() is not None:
            print("server already closed")
        else:
            try:
                proxy.quit({})
                slept = 0
                while srv.poll() is None:
                    # still running after 5 seconds
                    if slept >= 2:
                        print("close failed, terminating server")
                        srv.terminate()
                        break
                    else:
                        time.sleep(0.01)
                        slept += 0.01
            except Exception as e:
                print(f"quit errored: {e}\nterminating server")
                srv.terminate()
            finally:
                slept = 0
                while srv.poll() is None:
                    # still running 5 seconds after termination
                    if slept >= 5:
                        print("termination failed, killing server")
                        srv.kill()
                        break
                    else:
                        time.sleep(0.01)
                        slept += 0.01
        print("finished")

    atexit.register(close_server)
    return proxy
