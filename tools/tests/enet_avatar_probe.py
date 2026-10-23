"""Two-process loopback probe of the game's ENet DLL; never starts the game.

Uses a probe-only framing format, not the game's serialized user/image tokens.
No images, runtime logs or commands are written to disk.
"""
import argparse
import ctypes as c
import hashlib
import json
import multiprocessing as mp
import struct
import time
import zlib
from pathlib import Path


class Address(c.Structure):
    _fields_ = [('ip', c.c_ubyte * 16), ('port', c.c_ushort)]


class Event(c.Structure):
    _fields_ = [('type', c.c_int), ('peer', c.c_void_p),
                ('channel', c.c_ubyte), ('data', c.c_uint), ('packet', c.c_void_p)]


def library(path):
    dll = c.CDLL(path)
    signatures = {
        'initialize': (c.c_int, []),
        'deinitialize': (None, []),
        'address_set_ip': (c.c_int, [c.POINTER(Address), c.c_char_p]),
        'host_create': (c.c_void_p, [c.c_void_p, c.c_size_t, c.c_size_t,
                                  c.c_uint, c.c_uint, c.c_int]),
        'host_connect': (c.c_void_p, [c.c_void_p, c.POINTER(Address), c.c_size_t, c.c_uint]),
        'host_service': (c.c_int, [c.c_void_p, c.POINTER(Event), c.c_uint]),
        'host_flush': (None, [c.c_void_p]),
        'host_destroy': (None, [c.c_void_p]),
        'packet_create': (c.c_void_p, [c.c_char_p, c.c_size_t, c.c_uint]),
        'packet_get_data': (c.c_void_p, [c.c_void_p]),
        'packet_get_length': (c.c_int, [c.c_void_p]),
        'packet_dispose': (None, [c.c_void_p]),
        'peer_send': (c.c_int, [c.c_void_p, c.c_ubyte, c.c_void_p]),
    }
    for name, (result, args) in signatures.items():
        fn = getattr(dll, 'enet_' + name)
        fn.restype, fn.argtypes = result, args
    return dll


def worker(dll_path, port, channels, role, image, user_id, ready, results):
    dll = library(dll_path)
    host = None
    try:
        assert dll.enet_initialize() == 0
        address = Address()
        address.port = port
        assert dll.enet_address_set_ip(c.byref(address), b'127.0.0.1') == 0
        host = dll.enet_host_create(c.byref(address) if role == 'host' else None,
                                    1, 0, 0, 0, 0)
        assert host
        if role == 'host':
            ready.set()
        else:
            assert ready.wait(5)
            assert dll.enet_host_connect(host, c.byref(address), channels, 0)
        received, total, remote_id = {}, None, None
        stats = dict(role=role, pid=mp.current_process().pid, connected=False,
                     identity_received=False, image_send_ok=0, image_send_failed=0,
                     image_received=False)

        def send(peer, channel, payload):
            packet = dll.enet_packet_create(payload, len(payload), 1)
            assert packet
            code = dll.enet_peer_send(peer, channel, packet)
            if code != 0:
                dll.enet_packet_dispose(packet)
            return code

        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            event = Event()
            code = dll.enet_host_service(host, c.byref(event), 5)
            assert code >= 0
            if not code:
                continue
            if event.type == 1:
                stats['connected'] = True
                assert send(event.peer, 0, b'U' + user_id.encode()) == 0
                compressed = zlib.compress(image)
                for offset in range(0, len(compressed), 900):
                    payload = b'I' + struct.pack('<II', offset, len(compressed)) + compressed[offset:offset+900]
                    status = send(event.peer, 1, payload)
                    stats['image_send_ok' if status == 0 else 'image_send_failed'] += 1
                dll.enet_host_flush(host)
            elif event.type == 3:
                try:
                    data = c.string_at(dll.enet_packet_get_data(event.packet),
                                       dll.enet_packet_get_length(event.packet))
                    if data[:1] == b'U':
                        remote_id = data[1:].decode()
                        stats['identity_received'] = True
                    elif data[:1] == b'I':
                        offset, total = struct.unpack('<II', data[1:9])
                        received[offset] = data[9:]
                finally:
                    dll.enet_packet_dispose(event.packet)
        if total is not None:
            assert sum(map(len, received.values())) == total
            restored = zlib.decompress(b''.join(received[k] for k in sorted(received)))
            assert restored == image
            stats['image_received'] = True
            stats['sha256'] = hashlib.sha256(restored).hexdigest()
        stats['same_identity'] = remote_id == user_id
        results.put(stats)
    except BaseException:
        import traceback
        results.put(dict(role=role, error=traceback.format_exc()))
    finally:
        if host:
            dll.enet_host_destroy(host)
        dll.enet_deinitialize()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dll', required=True)
    parser.add_argument('--avatar', required=True, help='Existing avatar image; read only')
    parser.add_argument('--port', type=int, default=39471)
    args = parser.parse_args()
    image = Path(args.avatar).read_bytes()
    print(json.dumps(dict(image_bytes=len(image), sha256=hashlib.sha256(image).hexdigest())))
    for channels in (0, 2):
        ready, results = mp.Event(), mp.Queue()
        children = [mp.Process(target=worker, args=(args.dll, args.port, channels,
                    role, image, 'same-test-user', ready, results)) for role in ('host', 'client')]
        for child in children:
            child.start()
        for child in children:
            child.join(12)
            if child.is_alive():
                child.terminate()
                child.join()
                raise RuntimeError('Probe process timed out')
            assert child.exitcode == 0, child.exitcode
        rows = [results.get(timeout=3) for _ in children]
        print(json.dumps(dict(connect_channel_count=channels, results=rows), sort_keys=True))
        for row in rows:
            assert 'error' not in row, row
            assert row['connected'] and row['identity_received'] and row['same_identity']
            assert row['image_received'] == (channels == 2)
            assert row['image_send_failed'] > 0 if channels == 0 else row['image_send_failed'] == 0
    print('PASS: same-ID avatar loss reproduced; explicit two-channel connection transfers identical image both ways.')


if __name__ == '__main__':
    mp.freeze_support()
    main()
