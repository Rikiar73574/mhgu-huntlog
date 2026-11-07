"""Render the real dashboard in Qt; run under xvfb-run on Linux CI."""
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from desktop import JournalServer, create_window

try:
    from PySide6.QtCore import QEventLoop, QSettings, QTimer
    from PySide6.QtTest import QTest
    from PySide6.QtWidgets import QApplication, QDialog, QTextBrowser
except ImportError:
    QApplication = None


def javascript(view, source):
    loop = QEventLoop()
    values = []
    timer = QTimer()
    timer.setSingleShot(True)
    timer.timeout.connect(loop.quit)
    def receive(value):
        values.append(value)
        loop.quit()
    timer.start(15000)
    view.page().runJavaScript(source, receive)
    loop.exec()
    timer.stop()
    if not values:
        raise TimeoutError('Qt dashboard did not answer JavaScript')
    return values[0]


def wait_for(view, predicate):
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if javascript(view, predicate):
            return
        QTest.qWait(50)
    raise AssertionError('Dashboard did not finish rendering: ' + predicate)


@unittest.skipUnless(QApplication, 'desktop dependencies not installed')
class QtDesktopTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication(['HuntLog-test'])
        cls.app.setApplicationName('HuntLog-test')
        cls.app.setOrganizationName('HuntLog-test')

    def test_native_window_history_chart_filters_switch_and_settings(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            first, second = [root / name for name in ('one.jsonl', 'two.jsonl')]
            events = [dict(event='start', t=0),
                dict(event='sample', t=0, players=[dict(id=1, name='Test hunter', weapon_type=3)], monsters=[dict(id=1, name='Arzuros', hp=100, max_hp=100)]),
                dict(event='hit', t=2, player=1, monster=1, damage=15),
                dict(event='sample', t=10, players=[], monsters=[dict(id=1, hp=0, max_hp=100)]),
                dict(event='end', t=10)]
            first.write_text(''.join(json.dumps(dict(schema=1, run=7, **row))+'\n' for row in events))
            second.write_text(''.join(json.dumps(dict(schema=1, run=9, **row))+'\n' for row in events))
            settings = QSettings(str(root / 'app.ini'), QSettings.Format.IniFormat)
            with JournalServer(first) as server:
                window = create_window(server, settings)
                window.show()
                try:
                    wait_for(window.view, "document.querySelector('#notice')?.textContent.includes('1 saved runs')")
                    javascript(window.view, "document.querySelector('#nav-hunts').click()")
                    wait_for(window.view, "document.querySelectorAll('#runs button').length === 1")
                    javascript(window.view, "document.querySelector('#runs button').click()")
                    wait_for(window.view, "document.querySelectorAll('#graph path').length > 0")
                    self.assertIn('Arzuros', javascript(window.view, "document.querySelector('#detail').textContent"))
                    javascript(window.view, "document.querySelector('#search').value='no match'; document.querySelector('#search').dispatchEvent(new Event('input'))")
                    wait_for(window.view, "document.querySelectorAll('#runs button').length === 0")
                    with patch('PySide6.QtWidgets.QFileDialog.getOpenFileName', return_value=(str(second), '')):
                        window.open_journal()
                    wait_for(window.view, "document.querySelector('#notice')?.textContent.includes('1 saved runs')")
                    wait_for(window.view, "document.querySelector('#runs button')?.dataset.run === '9'")
                    self.assertEqual(server.journal, second)
                    self.assertEqual(settings.value('journal'), str(second))
                    self.assertIn(str(second), window.statusBar().currentMessage())
                    screenshot = os.environ.get('HUNTLOG_TEST_SCREENSHOT')
                    if screenshot:
                        javascript(window.view, "document.querySelector('#nav-hunts').click(); document.querySelector('#runs button').click()")
                        QTest.qWait(250)
                        self.assertTrue(window.grab().save(screenshot))
                finally:
                    window.close()
                    QTest.qWait(50)
                self.assertIsNotNone(settings.value('window_geometry'))
            self.assertFalse(server.thread.is_alive())

    def test_help_opens_bundled_third_party_licenses_without_replacing_dashboard(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            journal = root / 'empty.jsonl'
            journal.write_text('')
            settings = QSettings(str(root / 'app.ini'), QSettings.Format.IniFormat)
            with JournalServer(journal) as server:
                window = create_window(server, settings)
                window.show()
                try:
                    wait_for(window.view, "document.querySelector('#notice')?.textContent.includes('0 saved runs')")
                    dashboard_url = window.view.url()
                    help_action = next((a for a in window.menuBar().actions() if a.text() == '&Help'), None)
                    self.assertIsNotNone(help_action, 'The Help menu must expose bundled license credits')
                    credits_action = next((a for a in help_action.menu().actions() if 'licenses' in a.text()), None)
                    self.assertIsNotNone(credits_action, 'Help must open the bundled third-party license texts')
                    credits_action.trigger()
                    dialog = window.findChild(QDialog)
                    self.assertIsNotNone(dialog)
                    credits = dialog.findChild(QTextBrowser)
                    self.assertIsNotNone(credits)
                    text = credits.toPlainText()
                    for expected in ['Chromium', 'Copyright', 'zlib', 'Redistribution and use in source and binary forms']:
                        self.assertIn(expected, text)
                    self.assertTrue(credits.source().isLocalFile())
                    self.assertTrue(Path(credits.source().toLocalFile()).is_file())
                    self.assertTrue(dialog.isVisible())
                    self.assertEqual(window.view.url(), dashboard_url)
                    self.assertEqual(javascript(window.view, "document.querySelector('#notice').textContent.includes('0 saved runs')"), True)
                    window.close()
                    QTest.qWait(50)
                    self.assertFalse(any(child.isVisible() for child in window.findChildren(QDialog)),
                                     'Closing HuntLog must also close its license dialog')
                finally:
                    window.close()
                    QTest.qWait(50)
            self.assertFalse(server.thread.is_alive())

if __name__ == '__main__':
    unittest.main()
