#!/bin/python

import json
import requests
import sys


port = "8080"
print_success = True
command = "run"
test_id = -1


def red(str):
    return f"\033[91m{str}\033[0m"

def green(str):
    return f"\033[92m{str}\033[0m"

def blue(str):
    return f"\033[94m{str}\033[0m"

def run_test(idx, test):
    try: 
        resp = requests.post(f"http://localhost:{port}/move", json=test["payload"])
        move = resp.json()["move"]
        if move in test["allowed_moves"]:
            if print_success:
                print(f"{idx : >3}){test['comment'] : >75} : {green('ok')}")
        else:
            print(f"{idx : >3}){test['comment'] : >75} : {red('failed')}: {blue(move) : <14} instead of {test['allowed_moves']}")
    except Exception as e:
        print(e)

def run_tests(test_id):
    with open("./tests.json", "r") as f:
        tests = json.load(f)

    if test_id != -1:
        run_test(test_id, tests[test_id])
        return

    for i, test in enumerate(tests):
        run_test(i, test)

def show_test(idx):
    pass
    


# parse command line arguments
if sys.argv[1] == "show":
    show(sys.argv[2])
    exit()
i = 1
while i < len(sys.argv):
    arg = sys.argv[i]
    if arg == "-p" or arg == "--port":
        i += 1
        port = sys.argv[i]
    elif arg == "-f" or arg == "--fail-only":
        print_success = False
    elif arg == "-x" or arg == "--number":
        i += 1
        test_id = int(sys.argv[i])
    i += 1

run_tests(test_id)
