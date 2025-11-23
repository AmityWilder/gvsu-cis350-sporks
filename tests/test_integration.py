import time, xmlrpc.client, subprocess, atexit, datetime


def open_server(socket="127.0.0.1:8080"):
    # open the server in parallel
    srv = subprocess.Popen(["./target/debug/gvsu-cis350-sporks", socket])
    time.sleep(0.01) # wait for the server to be open

    proxy = xmlrpc.client.ServerProxy(f"http://{socket}")

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


def test_add_one_user():
    with open_server("127.0.0.1:8080") as proxy:
        user = {
            'name': "Squeeby Deeby",
        }
        ids = proxy.add_users([user])
        assert ids is not None and len(ids) == 1


def test_add_multiple_users():
    with open_server("127.0.0.1:8081") as proxy:
        user1 = {
            'name': "Dill Pickle",
        }
        user2 = {
            'name': "Mark Hapenstance",
        }
        ids = proxy.add_users([user1, user2])
        assert ids is not None and len(ids) == 2


def test_add_one_slot():
    with open_server("127.0.0.1:8082") as proxy:
        proxy.add_slots([{
            'start': datetime.datetime(2006, 11, 21, 16, 30),
            'end': datetime.datetime(2006, 11, 21, 16, 45),
            'min_staff': 3,
            'name': "Sweep dishes",
        }])
