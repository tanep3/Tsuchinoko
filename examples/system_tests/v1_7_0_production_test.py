import sys
import json
import subprocess
import unittest
import uuid

# Target Production Worker
WORKER_SCRIPT = "src/bridge/python/v1_7_0_worker.py"

class ProductionWorkerTest(unittest.TestCase):
    def setUp(self):
        self.process = subprocess.Popen(
            [sys.executable, WORKER_SCRIPT],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=sys.stderr, # Keep error visible
            text=True,
            bufsize=1 
        )
        self.session_id = str(uuid.uuid4())

    def tearDown(self):
        if self.process.poll() is None:
            self.process.terminate()
            self.process.wait()

    def send_request(self, cmd):
        json_line = json.dumps(cmd)
        self.process.stdin.write(json_line + "\n")
        self.process.stdin.flush()
        
        response_line = self.process.stdout.readline()
        if not response_line:
            raise EOFError("Worker closed stdout")
        return json.loads(response_line)

    def test_bootstrap_and_flow(self):
        # 1. Bootstrap: Create string handle using 'str'
        # Assumes 'str' is resolvable (builtins)
        req_create = {
            "cmd": "call_function",
            "session_id": self.session_id,
            "req_id": "r1",
            "target": "builtins.str",
            "args": [{"kind": "value", "value": "Production Ready"}]
        }
        resp = self.send_request(req_create)
        self.assertEqual(resp["kind"], "ok")
        h_str = resp["value"]
        self.assertEqual(h_str["kind"], "handle")
        h_id = h_str["id"]
        
        # 2. Call Method: .upper()
        req_call = {
            "cmd": "call_method",
            "session_id": self.session_id,
            "req_id": "r2",
            "target": h_id,
            "method": "upper",
            "args": []
        }
        resp = self.send_request(req_call)
        self.assertEqual(resp["kind"], "ok")
        self.assertEqual(resp["value"]["value"], "PRODUCTION READY")

    def test_import_math(self):
        # 1. Call math.pow(2, 3)
        req_call = {
            "cmd": "call_function",
            "session_id": self.session_id,
            "req_id": "m1",
            "target": "math.pow",
            "args": [{"kind": "value", "value": 2}, {"kind": "value", "value": 3}]
        }
        resp = self.send_request(req_call)
        self.assertEqual(resp["kind"], "ok")
        self.assertEqual(resp["value"]["value"], 8.0)

    def test_slice_failure_step_zero(self):
        # Create string
        req_create = {
            "cmd": "call_function",
            "session_id": self.session_id,
            "target": "builtins.str",
            "args": [{"kind": "value", "value": "ABC"}]
        }
        h_id = self.send_request(req_create)["value"]["id"]
        
        # Slice with step 0
        req_slice = {
            "cmd": "slice",
            "session_id": self.session_id,
            "target": h_id,
            "start": {"kind":"value", "value":None},
            "stop": {"kind":"value", "value":None},
            "step": {"kind":"value", "value":0}
        }
        resp = self.send_request(req_slice)
        self.assertEqual(resp["kind"], "error")
        # Spec says PythonException(ValueError)
        self.assertEqual(resp["error"]["code"], "PythonException")
        self.assertEqual(resp["error"]["py_type"], "ValueError")

if __name__ == "__main__":
    unittest.main()
