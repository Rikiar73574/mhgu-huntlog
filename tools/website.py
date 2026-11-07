"""HuntLog website launcher. No game or save files are modified."""
import argparse
import sys
import webbrowser
from pathlib import Path
from http.server import ThreadingHTTPServer
from serve_history import handler, DEFAULT

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('journal_file', nargs='?', type=Path, help='You may also drag hunter_hunts.jsonl onto the executable')
    parser.add_argument('--journal', type=Path)
    parser.add_argument('--port', type=int, default=8768)
    parser.add_argument('--no-browser', action='store_true')
    args = parser.parse_args()
    journal = args.journal or args.journal_file or DEFAULT
    try:
        server = ThreadingHTTPServer(('127.0.0.1', args.port), handler(journal))
    except OSError:
        server = ThreadingHTTPServer(('127.0.0.1', 0), handler(journal))
    url = f'http://127.0.0.1:{server.server_port}/'
    print(f'HuntLog website\n{url}\n\nReading: {journal}\n', flush=True)
    if not journal.is_file():
        print('No hunt log found yet. Complete a hunt with HuntLog installed.\nFor a different save location, drag hunter_hunts.jsonl onto this EXE.\n', flush=True)
    print('Keep this window open while using the website. Close it to stop.\n', flush=True)
    if not args.no_browser:
        webbrowser.open(url)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()

if __name__ == '__main__':
    main()
