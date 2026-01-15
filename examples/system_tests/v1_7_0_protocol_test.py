import sys
import json
import subprocess
import unittest
import uuid
from typing import Dict, Any

# Target Worker to test (Prototype for now)
WORKER_SCRIPT = "examples/verification/v1_7_0_worker_proto.py"

class ProtocolTest(unittest.TestCase):
    def setUp(self):
        # Launch the worker process
        self.process = subprocess.Popen(
            [sys.executable, WORKER_SCRIPT],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=sys.stderr,
            text=True,
            bufsize=1 # Line buffered
        )
        self.session_id = str(uuid.uuid4())

    def tearDown(self):
        if self.process.poll() is None:
            self.process.terminate()
            self.process.wait()

    def send_request(self, cmd: Dict[str, Any]) -> Dict[str, Any]:
        """Send a JSON command to the worker and return the response."""
        json_line = json.dumps(cmd)
        self.process.stdin.write(json_line + "\n")
        self.process.stdin.flush()
        
        response_line = self.process.stdout.readline()
        if not response_line:
            raise EOFError("Worker closed stdout")
        return json.loads(response_line)

    def test_basic_flow(self):
        """Test a basic flow: Create (via direct internal logic mock) -> Call Method -> Delete"""
        # Note: In the prototype, we assume we can "load" something or utilize built-ins for testing.
        # Since 'import' logic is part of the worker initialization or specific commands,
        # for this test we'll rely on the prototype having some built-in capability or mocked object.
        
        # Ideally, we should have a 'create_demo_object' command or similar for testing, 
        # or we assume a handle 'demo_obj' exists if the prototype is purely for this test.
        # But per specs, handles come from imports or returns.
        
        # For this system test, we will assume the worker can handle a special debug command or
        # we try to call a method on a string literal if supported, OR we just check a simple Ping-pong if defined.
        
        # Let's assume the worker implements a simple 'string' wrapping for testing 'call_method'.
        # But wait, we don't have a command to *create* a handle from scratch in the API spec explicitly 
        # (imports are done via Rust side bridge logic usually hidden from RPC).
        # However, Phase 0 prototype usually mocks imports or has a "hello" object.
        
        # Let's try to 'slice' a raw value, which is allowed by the spec (slice args can be values).
        # Actually slice target must be a handle.
        
        # STRATEGY: The prototype worker should allow creating a handle via a special backdoor command 
        # OR we implement 'eval' equivalent for testing locally? No, security violation.
        
        # Let's assume the prototype worker pre-loads a "math" module or similar under a known handle for testing.
        # OR, better: The real worker will manage imports.
        # For the prototype, let's allow a "debug_create_string" command to make a handle.
        
        # 1. Create Handle
        req_create = {
            "cmd": "debug_create_string",
            "session_id": self.session_id,
            "req_id": "req-1",
            "value": "Hello World"
        }
        # Worker should respond with a handle
        resp_create = self.send_request(req_create)
        self.assertEqual(resp_create["kind"], "ok")
        handle = resp_create["value"]
        self.assertEqual(handle["kind"], "handle")
        h_id = handle["id"]

        # 2. Call Method: .upper()
        req_call = {
            "cmd": "call_method",
            "session_id": self.session_id,
            "req_id": "req-2",
            "target": h_id,
            "method": "upper",
            "args": []
        }
        resp_call = self.send_request(req_call)
        self.assertEqual(resp_call["kind"], "ok")
        self.assertEqual(resp_call["req_id"], "req-2")
        self.assertEqual(resp_call["value"]["kind"], "value")
        self.assertEqual(resp_call["value"]["value"], "HELLO WORLD")
        
        # 3. Get Item (Index)
        req_item = {
            "cmd": "get_item",
            "session_id": self.session_id,
            "req_id": "req-3",
            "target": h_id,
            "key": {"kind": "value", "value": 1}
        }
        resp_item = self.send_request(req_item)
        self.assertEqual(resp_item["kind"], "ok")
        self.assertEqual(resp_item["value"]["value"], "e")

        # 4. Delete
        req_del = {
            "cmd": "delete",
            "session_id": self.session_id,
            "req_id": "req-4",
            "target": h_id
        }
        resp_del = self.send_request(req_del)
        self.assertEqual(resp_del["kind"], "ok")
        
        # 5. Access after delete (Should fail with StaleHandle)
        req_call_fail = {
            "cmd": "call_method",
            "session_id": self.session_id,
            "req_id": "req-5",
            "target": h_id,
            "method": "lower",
            "args": []
        }
        resp_fail = self.send_request(req_call_fail)
        self.assertEqual(resp_fail["kind"], "error")
        # In the spec we defined error code "StaleHandle"
        self.assertEqual(resp_fail["error"]["code"], "StaleHandle")

    def test_slice_command(self):
        # Create "0123456789"
        req_create = {
            "cmd": "debug_create_string",
            "session_id": self.session_id,
            "req_id": "s-1",
            "value": "0123456789"
        }
        h_id = self.send_request(req_create)["value"]["id"]
        
        # Slice [1:5:2] -> "13"
        req_slice = {
            "cmd": "slice",
            "session_id": self.session_id,
            "req_id": "s-2",
            "target": h_id,
            "start": {"kind": "value", "value": 1},
            "stop": {"kind": "value", "value": 5},
            "step": {"kind": "value", "value": 2}
        }
        resp = self.send_request(req_slice)
        self.assertEqual(resp["kind"], "ok")
        self.assertEqual(resp["value"]["value"], "13") # "1", "3" from "012345" indices 1,3

    def test_iter_command(self):
        # Create list [1, 2, 3]
        # Assuming debug_eval or similar for creating list
        req_create = {
            "cmd": "debug_eval",
            "session_id": self.session_id,
            "req_id": "i-1",
            "code": "[1, 2, 3, 4, 5]"
        }
        h_id = self.send_request(req_create)["value"]["id"]
        
        # Create Iterator
        req_iter = {
            "cmd": "iter",
            "session_id": self.session_id,
            "req_id": "i-2",
            "target": h_id
        }
        resp_iter = self.send_request(req_iter)
        self.assertEqual(resp_iter["kind"], "ok")
        iter_id = resp_iter["value"]["id"]
        
        # Next Batch (size 2) -> [1, 2]
        req_next = {
            "cmd": "iter_next_batch",
            "session_id": self.session_id,
            "req_id": "i-3",
            "target": iter_id,
            "batch_size": 2
        }
        resp_next = self.send_request(req_next)
        self.assertEqual(resp_next["kind"], "ok")
        self.assertEqual(resp_next["value"]["kind"], "list")
        items = resp_next["value"]["items"]
        self.assertEqual(len(items), 2)
        self.assertEqual(items[0]["value"], 1)
        self.assertEqual(items[1]["value"], 2)
        self.assertFalse(resp_next.get("meta", {}).get("done", False))

        # Next Batch (size 10) -> [3, 4, 5], done=True
        # Note: Depending on implementation, it might return [3,4,5] with done=False, then next call [] with done=True.
        # Or [3,4,5] with done=True if it knows it's exhausted.
        # Let's assume standard iterator behavior: might need one extra call or return remaining.
        
        req_next_2 = {
            "cmd": "iter_next_batch",
            "session_id": self.session_id,
            "req_id": "i-4",
            "target": iter_id,
            "batch_size": 10
        }
        resp_next_2 = self.send_request(req_next_2)
        self.assertEqual(resp_next_2["kind"], "ok")
        items_2 = resp_next_2["value"]["items"]
        # It should return the rest
        self.assertEqual(len(items_2), 3) # 3,4,5
        
        # Check if done is set. If not, next call must be empty and done.
        if not resp_next_2.get("meta", {}).get("done", False):
            req_next_3 = {
                "cmd": "iter_next_batch",
                "session_id": self.session_id,
                "req_id": "i-5",
                "target": iter_id,
                "batch_size": 10
            }
            resp_next_3 = self.send_request(req_next_3)
            self.assertEqual(resp_next_3["kind"], "ok")
            self.assertEqual(len(resp_next_3["value"]["items"]), 0)
            self.assertTrue(resp_next_3["meta"]["done"])

if __name__ == "__main__":
    unittest.main()
