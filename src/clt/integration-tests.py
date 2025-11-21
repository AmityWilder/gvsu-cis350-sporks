import time, xmlrpc.client, subprocess, atexit, datetime


def test_add_one_user(proxy):
    user = {
        'name': "Squeeby Deeby",
    }
    ids = proxy.add_users({'to_add': [user]})
    assert ids is not None and len(ids) == 1


def test_add_multiple_users(proxy):
    user1 = {
        'name': "Dill Pickle",
    }
    user2 = {
        'name': "Mark Hapenstance",
    }
    ids = proxy.add_users({'to_add': [user1, user2]})
    assert ids is not None and len(ids) == 2


def test_add_one_slot(proxy):
    proxy.add_slots([{
        'start': datetime.datetime(2006, 11, 21, 16, 30),
        'end': datetime.datetime(2006, 11, 21, 16, 45),
        'min_staff': 3,
        'name': "Sweep dishes",
    }])


TESTS = [
    ('test_add_one_slot', test_add_one_slot),
    ('test_add_one_user', test_add_one_user),
    ('test_add_multiple_users', test_add_multiple_users),
]


IS_DEBUG_BUILD = True
if IS_DEBUG_BUILD:
    BUILD = "debug"
else:
    BUILD = "release"

# open the server in parallel
srv = subprocess.Popen([f"./target/{BUILD}/gvsu-cis350-sporks"])
time.sleep(0.01) # wait for the server to be open

# create a line of communication with the server
with xmlrpc.client.ServerProxy("http://127.0.0.1:8080", use_datetime=True) as proxy:

    def close_server():
        print("attempting to close server")
        if srv.poll() is not None:
            return # server has already closed
        try:
            proxy.quit({})
        except:
            pass # ignore failure, we need to kill it if it won't go quietly
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

    for name,f in TESTS:
        print(name, end=': ')
        try:
            f(proxy)
            print("passed")
        except Exception as e:
            print(f"failed\n  {e}")
