from server import *
import threading


address = 8080
address_lock = threading.Lock()

def open_test_server():
    """Choose the next socket address so tests running in parallel dont try to share a server"""
    address_lock.acquire()
    global address
    proxy = open_server("debug")
    address += 1
    address_lock.release()
    return proxy


def test_pat_starts_with():
    with open_test_server() as proxy:
        pat = proxy.pat_starts_with("squeak")
        assert pat is not None


def test_add_zero_users():
    with open_test_server() as proxy:
        ids = proxy.add_users([])
        assert ids is not None and len(ids) == 0


def test_add_one_user():
    with open_test_server() as proxy:
        user = {
            'name': "Squeeby Deeby",
        }
        ids = proxy.add_users([user])
        assert ids is not None and len(ids) == 1


def test_add_multiple_users():
    with open_test_server() as proxy:
        user1 = {
            'name': "Dill Pickle",
        }
        user2 = {
            'name': "Mark Hapenstance",
        }
        ids = proxy.add_users([user1, user2])
        assert ids is not None and len(ids) == 2


def test_add_zero_slots():
    with open_test_server() as proxy:
        proxy.add_slots([])


def test_add_one_slot():
    with open_test_server() as proxy:
        slot = {
            'start': datetime(2006, 11, 21, 16, 30),
            'end': datetime(2006, 11, 21, 16, 45),
            'min_staff': 3,
            'name': "Morning Shift",
        }
        proxy.add_slots([slot])


def test_add_multiple_slots():
    with open_test_server() as proxy:
        slot1 = {
            'start': datetime(2006, 11, 21, 16, 30),
            'end': datetime(2006, 11, 21, 16, 45),
            'min_staff': 3,
            'name': "Morning Shift",
        }
        slot2 = {
            'start': datetime(2006, 11, 22, 13, 30),
            'end': datetime(2006, 11, 23, 15, 00),
            'min_staff': 2,
            'name': "Noon Shift",
        }
        proxy.add_slots([slot1, slot2])


def test_add_zero_tasks():
    with open_test_server() as proxy:
        proxy.add_tasks([])


def test_add_one_task():
    with open_test_server() as proxy:
        task = {
            'title': "Sweep dishes",
            'desc': "you heard me",
            'deadline': datetime(2006, 11, 23, 15, 00),
            'awaiting': [],
        }
        proxy.add_tasks([task])


def test_add_multiple_tasks():
    with open_test_server() as proxy:
        task1 = {
            'title': "Sweep dishes",
            'desc': "you heard me",
            'deadline': datetime(2006, 11, 23, 15, 00),
            'awaiting': [],
        }
        task2 = {
            'title': "Build soap",
            'desc': "you heard me",
            'deadline': datetime(2006, 11, 23, 15, 00),
            'awaiting': [],
        }
        proxy.add_tasks([task1, task2])
