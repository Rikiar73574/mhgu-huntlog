"""HuntLog desktop dashboard. Reads hunt journals without modifying game files."""
import argparse
from http.server import ThreadingHTTPServer
from pathlib import Path
import sys
from threading import Thread

from history_paths import HISTORY, default_journal
from serve_history import handler


class JournalServer:
    """One private loopback listener; changing the journal needs no new process."""
    def __init__(self, journal):
        self.journal = Path(journal)
        self.server = ThreadingHTTPServer(('127.0.0.1', 0), handler(lambda: self.journal))
        self.server.daemon_threads = True
        self.address = self.server.server_address
        self.url = f'http://127.0.0.1:{self.address[1]}/'
        self.thread = Thread(target=self.server.serve_forever, name='HuntLog journal', daemon=True)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *_):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()


def create_window(server, settings):
    from PySide6.QtCore import Qt, QUrl
    from PySide6.QtGui import QAction, QDesktopServices, QIcon, QKeySequence
    from PySide6.QtWidgets import QDialog, QFileDialog, QMainWindow, QMessageBox, QTextBrowser, QVBoxLayout
    from PySide6.QtWebEngineCore import QWebEnginePage, QWebEngineSettings
    from PySide6.QtWebEngineWidgets import QWebEngineView

    class LocalPage(QWebEnginePage):
        def acceptNavigationRequest(self, url, kind, main_frame):
            if not main_frame or url.toString().startswith(server.url):
                return True
            if url.scheme() in ('http', 'https'):
                QDesktopServices.openUrl(url)
            return False

    class Window(QMainWindow):
        def __init__(self):
            super().__init__()
            self.setWindowTitle('HuntLog')
            self.setWindowIcon(QIcon(str(HISTORY / 'huntlog.svg')))
            self.resize(1240, 860)
            self.setMinimumSize(800, 600)
            self.view = QWebEngineView(self)
            self.view.setPage(LocalPage(self.view))
            self.view.settings().setAttribute(QWebEngineSettings.WebAttribute.LocalContentCanAccessRemoteUrls, False)
            self.setCentralWidget(self.view)
            self.view.setUrl(QUrl(server.url + '#home'))
            file_menu = self.menuBar().addMenu('&File')
            self.action(file_menu, '&Open hunt journal…', QKeySequence.StandardKey.Open, self.open_journal)
            self.action(file_menu, 'Show journal &folder', None, lambda: QDesktopServices.openUrl(QUrl.fromLocalFile(str(server.journal.parent))))
            file_menu.addSeparator()
            self.action(file_menu, '&Quit', QKeySequence.StandardKey.Quit, self.close)
            view_menu = self.menuBar().addMenu('&View')
            self.action(view_menu, '&Refresh', QKeySequence.StandardKey.Refresh, self.view.reload)
            self.action(view_menu, 'Zoom &in', QKeySequence.StandardKey.ZoomIn, lambda: self.zoom(.1))
            self.action(view_menu, 'Zoom &out', QKeySequence.StandardKey.ZoomOut, lambda: self.zoom(-.1))
            self.action(view_menu, '&Actual size', 'Ctrl+0', lambda: self.view.setZoomFactor(1))
            help_menu = self.menuBar().addMenu('&Help')
            self.action(help_menu, 'About &Qt', None, lambda: QMessageBox.aboutQt(self, 'About Qt'))
            self.action(help_menu, 'Third-party &licenses', None, self.show_credits)
            geometry = settings.value('window_geometry')
            if geometry is not None:
                self.restoreGeometry(geometry)
            self.show_journal()

        def action(self, menu, title, shortcut, callback):
            action = QAction(title, self)
            if shortcut is not None:
                action.setShortcut(shortcut)
            action.triggered.connect(callback)
            menu.addAction(action)

        def zoom(self, amount):
            self.view.setZoomFactor(max(.5, min(2, self.view.zoomFactor() + amount)))

        def show_credits(self):
            dialog = QDialog(self)
            dialog.setWindowTitle('Third-party licenses')
            dialog.setAttribute(Qt.WidgetAttribute.WA_DeleteOnClose)
            dialog.resize(900, 700)
            layout = QVBoxLayout(dialog)
            view = QTextBrowser(dialog)
            view.setOpenExternalLinks(True)
            layout.addWidget(view)
            base = Path(sys.executable).resolve().parent if getattr(sys, 'frozen', False) else HISTORY.parent
            view.setSource(QUrl.fromLocalFile(str(base / 'licenses/Qt-WebEngine/index.html')))
            dialog.show()

        def show_journal(self):
            prefix = '' if server.journal.is_file() else 'No journal found. File > Open hunt journal: '
            self.statusBar().showMessage(prefix + str(server.journal))

        def open_journal(self):
            path, _ = QFileDialog.getOpenFileName(self, 'Open hunt journal', str(server.journal.parent), 'Hunt journals (*.jsonl);;All files (*)')
            if path:
                server.journal = Path(path)
                settings.setValue('journal', path)
                self.show_journal()
                self.view.reload()

        def closeEvent(self, event):
            for dialog in self.findChildren(QDialog):
                dialog.close()
            settings.setValue('window_geometry', self.saveGeometry())
            settings.sync()
            super().closeEvent(event)

    return Window()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('journal_file', nargs='?', type=Path, help='Hunt journal, also accepted by dragging it onto the executable')
    parser.add_argument('--journal', type=Path)
    parser.add_argument('--smoke-report', type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)
    from PySide6.QtCore import QSettings
    from PySide6.QtWidgets import QApplication
    from PySide6.QtGui import QPalette, QColor
    app = QApplication([sys.argv[0]])
    app.setApplicationName('HuntLog')
    app.setOrganizationName('HuntLog')
    app.setStyle('Fusion')
    palette = app.palette()
    for role, color in [(QPalette.ColorRole.Window, '#181311'), (QPalette.ColorRole.Base, '#181311'),
                        (QPalette.ColorRole.Button, '#2b201b'), (QPalette.ColorRole.WindowText, '#f7eee1'),
                        (QPalette.ColorRole.Text, '#f7eee1'), (QPalette.ColorRole.ButtonText, '#f7eee1'),
                        (QPalette.ColorRole.Highlight, '#685327')]:
        palette.setColor(role, QColor(color))
    app.setPalette(palette)
    settings = QSettings()
    journal = args.journal or args.journal_file or Path(settings.value('journal', str(default_journal())))
    with JournalServer(journal) as server:
        window = create_window(server, settings)
        window.show()
        if args.smoke_report:
            from PySide6.QtCore import QTimer
            import json
            timer = QTimer(window)
            attempts = 0
            def checked(result):
                if not result:
                    return
                state = json.loads(result)
                if not state['ready']:
                    return
                timer.stop()
                args.smoke_report.write_text(json.dumps(state, indent=2) + '\n')
                window.grab().save(str(args.smoke_report.with_suffix('.png')))
                window.close()
                app.exit(0 if state['graph'] else 1)
            def check():
                nonlocal attempts
                attempts += 1
                if attempts > 60:
                    timer.stop()
                    args.smoke_report.write_text(json.dumps({'ready': False, 'error': 'Dashboard load timeout'}) + '\n')
                    window.close()
                    app.exit(1)
                    return
                window.view.page().runJavaScript("""(() => {
                    const ready = document.querySelector('#notice')?.textContent.includes('saved runs');
                    if (ready) {
                        document.querySelector('#nav-hunts').click();
                        document.querySelector('#runs button')?.click();
                    }
                    return JSON.stringify({ready: !!ready, graph: document.querySelectorAll('#graph path').length > 0,
                        title: document.title, runs: document.querySelectorAll('#runs button').length});
                })()""", checked)
            timer.timeout.connect(check)
            timer.start(500)
        return app.exec()


if __name__ == '__main__':
    sys.exit(main())
