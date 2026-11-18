"""Exact message acceptance by real Claude using a private home and localhost API."""
import re
import sys
import time
import unittest
import json
import test_input as fixtures
import test_claude_questions_native as native


@unittest.skipUnless(native.CLAUDE, 'Set HGS_CLAUDE_TEST_BIN for isolated native message coverage')
class NativeClaudeInput(unittest.TestCase):
    message_fixture = True
    native_env = ('DISABLE_AUTOUPDATER=1',)
    def native_settings(self, work):
        if 'pending_settings_then_message' not in self._testMethodName:
            return {}
        import copy
        from claude_catalog_fixture import CatalogFixture
        from test_claude_settings_native import CATALOG, EFFORTS
        catalog = copy.deepcopy(CATALOG)
        catalog['catalog']['config']['models'].append(dict(
            id='claude-fable-5-2', name='Fable 5.2', short_name='Fable', section='overflow',
            thinking=dict(type='effort', effort_options=EFFORTS)))
        (self.root / '.claude/cache/model-catalog').mkdir(parents=True)
        self.catalog_fixture = CatalogFixture(self.root, catalog)
        self.addCleanup(self.catalog_fixture.close)
        self.catalog_fixture.seed(self.root)
        self.native_env = (*self.native_env, *[key + '=' + value for key, value in self.catalog_fixture.environment().items()])
        return dict(model='opus', effortLevel='medium')
    def setUp(self):
        native.NativeClaudeQuestions.setUp(self)
        dismissed = False
        def ready():
            nonlocal dismissed
            screen = self.screen()
            if not dismissed and 'Make auto mode your default permission mode?' in screen and 'No, keep' in screen:
                self.tmux('send-keys', '-t', self.pane, 'Down')
                self.wait_for(lambda: re.search(r'❯ No, keep', self.screen()))
                time.sleep(.2)
                self.tmux('send-keys', '-t', self.pane, 'Enter')
                dismissed = True
                return False
            return re.search(r'^❯\s*$', screen, re.M)
        self.wait_for(ready)
    script = fixtures.InputTransport.script
    tmux = fixtures.InputTransport.tmux
    write_record = fixtures.InputTransport.write_record
    wait_for = native.NativeClaudeQuestions.wait_for
    screen = native.NativeClaudeQuestions.screen
    send = fixtures.InputTransport.send
    payload = fixtures.InputTransport.payload

    def accepted(self, text):
        return sum(row.get('type') == 'user' and row.get('message', {}).get('content') == text
                   for row in native.records(self.record['transcript']))

    def test_trailing_backslash_starts_one_turn_with_the_exact_message(self):
        text = 'Continue the fixture\\'
        payload = self.payload(text=text)
        try:
            receipt = self.send(payload)
        except AssertionError as error:
            raise AssertionError(str(error) + '\n' + self.screen()) from error
        self.assertEqual(receipt['text'], text)
        self.wait_for(lambda: self.accepted(text))
        self.assertEqual(self.accepted(text), 1)
        self.assertEqual(self.send(payload), receipt)
        self.assertEqual(self.accepted(text), 1)

    def test_multiline_message_ending_in_backslashes_is_preserved(self):
        text = 'Literal paths:\nfixture\\\\'
        self.send(self.payload(text=text))
        self.wait_for(lambda: self.accepted(text))

    def test_leading_absolute_path_is_literal_message_text(self):
        # A command-named directory must not invoke /help or file completion.
        text = '/help/fixture/capture.png\nWhat is shown in this screenshot?'
        payload = self.payload(text=text)
        receipt = self.send(payload)
        self.assertEqual(receipt['text'], text)
        self.wait_for(lambda: self.accepted(text))
        self.assertEqual(self.accepted(text), 1)

        self.assertEqual(self.send(payload), receipt)
        self.assertEqual(self.accepted(text), 1)

    def test_native_refusal_has_an_opted_in_terminal_outcome(self):
        import native_send_outcome
        native_send_outcome.refused_draft(self, self.name, self.record, self.screen,
                                         lambda: len(self.native_requests))

    def test_id_hidden_pending_settings_then_message_makes_one_model_call_and_keeps_defaults(self):
        self.pending_message()

    def pending_message(self):
        import native_effort_id as identity
        defaults = self.root / '.claude/settings.json'
        before = defaults.read_bytes()
        requests = len(self.native_requests)
        record, path, _ = identity.moved_record(self, self.name, self.record, existing_machine=True)
        self.record, self.record_path = record, path
        attempts = identity.key_log(self)
        text = 'Continue after the pending session model choice\\'
        receipt = identity.pending_then_message(self, self.name, record, path,
                                                'claude-fable-5-2', 'xhigh', text, attempts)
        self.wait_for(lambda: self.accepted(text))
        self.assertEqual(self.accepted(text), 1)
        self.assert_pending_requests(requests, text, 'claude-fable-5-2')
        self.assertEqual(defaults.read_bytes(), before)
        self.assertNotIn('Select model', self.screen())
        self.assertEqual(json.loads(path.read_bytes())['model'], 'claude-fable-5-2')
        return receipt, attempts

    def assert_pending_requests(self, before, text, model):
        self.wait_for(lambda: len(self.native_requests) >= before + 1)
        self.assertEqual(len(self.native_requests), before + 1, self.native_requests[before:])
        self.assertEqual(self.native_requests[-1]['model'], model)


# zerus-33caz: the shape of the stuck send, with synthetic identities. Three
# lines and under 800 characters, so Claude keeps it in the composer instead of
# a [Pasted text] placeholder, and taller than the composer of a 24-row pane:
# Claude then shows only its last rows under a "─── ↑ 3 more ───" bar. The
# Rust settle tests read the same message beside Claude's captured screens.
TALL = (fixtures.REPO / 'tests/fixtures/claude-composer-scrolled-message.txt').read_text()
LONG = 'Show the long fixture reply.'
SLOW = 'Start the slow fixture turn.'


@unittest.skipUnless(native.CLAUDE, 'Set HGS_CLAUDE_TEST_BIN for isolated native message coverage')
class NativeClaudeTallInput(NativeClaudeInput):
    """A message taller than the composer of an 80x24 pane (tmux's size for a
    session no client has attached to), sent once the conversation above has
    scrolled the earlier `❯` prompt out of view, as in the owner's pane."""
    test_trailing_backslash_starts_one_turn_with_the_exact_message = None
    test_multiline_message_ending_in_backslashes_is_preserved = None
    test_leading_absolute_path_is_literal_message_text = None
    test_native_refusal_has_an_opted_in_terminal_outcome = None
    test_id_hidden_pending_settings_then_message_makes_one_model_call_and_keeps_defaults = None

    def native_response_blocks(self, body):
        messages = body.get('messages') or [{}]
        last = json.dumps(messages[-1], ensure_ascii=False)
        if SLOW in last:
            # Held until the hgs paste lands, so the turn ends inside its window.
            release = self.root / 'turn-release'
            for _ in range(600):
                if release.exists():
                    break
                time.sleep(.05)
        if SLOW in last or LONG in last:
            return [dict(type='text', text='Fixture complete.' + ''.join(
                f'\n\nReply paragraph {n} of the long fixture answer.' for n in range(30)))]
        return [dict(type='text', text='Fixture complete.')]

    def ask(self, prompt):
        self.tmux('send-keys', '-t', self.pane, '-l', prompt)
        self.wait_for(lambda: prompt in self.screen())
        time.sleep(.2)
        self.tmux('send-keys', '-t', self.pane, 'Enter')
        self.wait_for(lambda: self.accepted(prompt))

    def replied(self):
        return any(row.get('type') == 'assistant' and 'Reply paragraph 29' in json.dumps(row)
                   for row in native.records(self.record['transcript']))

    def assert_confirmed(self, payload, proofs):
        try:
            receipt = self.send(payload)
        except AssertionError as error:
            raise AssertionError(str(error) + '\n' + self.screen()) from error
        self.assertEqual(receipt['status'], 'submitted', receipt)
        self.assertIn(receipt.get('native'), proofs, receipt)
        self.wait_for(lambda: self.accepted(TALL))
        self.assertEqual(self.accepted(TALL), 1)
        # Never resent: the same request returns its receipt.
        self.assertEqual(self.send(payload), receipt)
        self.assertEqual(self.accepted(TALL), 1)
        return receipt

    def test_tall_message_to_an_idle_claude_80x24_is_confirmed(self):
        self.ask(LONG)
        self.wait_for(lambda: self.replied() and re.search(r'^❯\s*$', self.screen(), re.M))
        # The earlier prompt row is out of view, as in the owner's pane.
        self.assertNotIn('❯ ' + LONG, self.screen())
        self.assert_confirmed(self.payload(text=TALL), ('transcript',))

    def test_tall_message_to_a_claude_finishing_its_turn_80x24_is_confirmed(self):
        # The fixture's tmux releases the held reply right after the paste:
        # Claude streams it and ends its turn while hgs waits for the composer.
        self.script('tmux', '#!/bin/sh\n' + fixtures.TMUX + ' -S "' + str(self.socket) + '" -f /dev/null "$@"\n'
                    'status=$?\n[ "$1" = paste-buffer ] && touch "' + str(self.root / 'turn-release') + '"\nexit $status\n')
        self.ask(LONG)
        self.wait_for(lambda: self.replied() and re.search(r'^❯\s*$', self.screen(), re.M))
        self.tmux('send-keys', '-t', self.pane, '-l', SLOW)
        self.wait_for(lambda: SLOW in self.screen())
        time.sleep(.2)
        self.tmux('send-keys', '-t', self.pane, 'Enter')
        self.wait_for(lambda: any(SLOW in json.dumps(request.get('messages', [])[-1:], ensure_ascii=False)
                                  for request in self.native_requests))
        self.wait_for(lambda: re.search(r'^❯\s*$', self.screen(), re.M))
        self.record.update(activity='busy', phase='working', last_event_at=time.time())
        self.write_record()
        try:
            self.assert_confirmed(self.payload(text=TALL), ('transcript', 'queue'))
        finally:
            (self.root / 'turn-release').touch()

    def test_a_draft_typed_after_the_send_gate_is_never_submitted_with_the_message(self):
        # zerus-7cq23: a person types `x` into Claude's composer right after
        # the send gate read it empty (the fixture's tmux types it once hgs has
        # staged its buffer, after the cursor read that ends the gate's frame).
        # Claude keeps the draft out of view above `↑ 3 more`: the same rows,
        # the same count and the same cursor as this message alone.
        self.script('tmux', DRAFT_TMUX % dict(python=sys.executable, root=str(self.root), tmux=fixtures.TMUX,
                                              socket=str(self.socket)))
        self.ask(LONG)
        self.wait_for(lambda: self.replied() and re.search(r'^❯\s*$', self.screen(), re.M))
        requests = len(self.native_requests)
        (self.root / 'draft-armed').touch()
        payload = self.payload(text=TALL)
        error = self.send(payload, success=False)
        calls = (self.root / 'tmux-calls.log').read_text()
        self.assertEqual(json.loads((self.root / 'draft-state.json').read_text())['step'], 'injected', calls)
        # Never Enter on the draft and the message together, nor on the message.
        self.assertEqual(self.accepted('x' + TALL), 0, error)
        self.assertEqual(self.accepted(TALL), 0, error)
        self.assertNotIn('paste-buffer', calls, error)
        self.assertIn('nothing was pasted', error)
        self.assertIn('the composer holds: x', error)
        # The person's draft stays as typed, and nothing reached the model.
        self.assertRegex(self.screen(), r'(?m)^❯\s*x\s*$')
        self.assertEqual(len(self.native_requests), requests)
        # Never replayed: the same request pastes nothing.
        self.send(payload, success=False)
        self.assertNotIn('paste-buffer', (self.root / 'tmux-calls.log').read_text())
        self.assertRegex(self.screen(), r'(?m)^❯\s*x\s*$')


# zerus-7cq23: the fixture's tmux for a person's draft typed between the send
# gate's last read and the paste. Armed by root/'draft-armed', it waits for hgs
# to stage its buffer (load-buffer), then for the gate's composer capture and
# the cursor read after it, types `x` once Claude shows it, and logs each call.
DRAFT_TMUX = r'''#!%(python)s
import json, subprocess, sys, time
from pathlib import Path
root = Path(%(root)r)
real = [%(tmux)r, '-S', %(socket)r, '-f', '/dev/null']
args = sys.argv[1:]
status = subprocess.run(real + args).returncode
if (root / 'draft-armed').exists():
    path = root / 'draft-state.json'
    state = json.loads(path.read_text()) if path.exists() else dict(step='armed')
    command = args[0] if args else ''
    if state['step'] == 'armed' and command == 'load-buffer':
        state['step'] = 'staged'
    elif state['step'] == 'staged' and command == 'capture-pane' and '-e' in args:
        state['step'] = 'captured'
    elif state['step'] == 'captured' and command == 'display-message' and any('#{cursor_x}' in arg for arg in args):
        pane = args[args.index('-t') + 1]
        subprocess.run(real + ['send-keys', '-t', pane, '-l', 'x'])
        for _ in range(500):
            screen = subprocess.run(real + ['capture-pane', '-p', '-t', pane], capture_output=True, text=True).stdout
            if any(row[:1] == '❯' and row[1:].strip() == 'x' for row in screen.splitlines()):
                break
            time.sleep(.01)
        state['step'] = 'injected'
    path.write_text(json.dumps(state))
    with open(root / 'tmux-calls.log', 'a') as log:
        log.write(' '.join(args[:2]) + '\n')
sys.exit(status)
'''
