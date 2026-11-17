"""Launch placement survives real catalog contention, using product writers.

The tracked parent record is the existing fixture's explicit metadata seam:
its actual PID is this controller, so hgs proves the launch ancestry itself.
All child bindings, parent links and catalog membership rows use hgs writers.
The native agents are scripted fixtures, not released providers/models.
"""
from concurrent.futures import ThreadPoolExecutor
import fcntl
import json
import subprocess
import time

import test_child_project as child


class PendingPlacement(child.Placement, child.Base):
    def setUp(self):
        child.Base.setUp(self)
        self.leads = []
        self.state = self.root / 'state'
        self.state.mkdir(exist_ok=True)
        self.config = self.root / 'cfg'
        self.folder = self.project.resolve()

    def tearDown(self):
        for path in self.leads:
            path.unlink(missing_ok=True)
        child.Base.tearDown(self)

    named = child.LifecycleChild.named
    binding = child.LifecycleChild.binding
    held_locks = child.LifecycleChild.held_locks
    launch = child.LifecycleChild.launch

    def worker(self):
        log = (self.root / 'catalog-worker.log').open('w')
        process = subprocess.Popen([str(child.HGS), 'swarm', 'worker'],
                                   env=self.env, stdout=log, stderr=subprocess.STDOUT)
        def stop():
            if process.poll() is None:
                process.terminate()  # Only this exact child we created.
                process.wait(timeout=10)
            log.close()
        self.addCleanup(stop)
        return process

    def contended(self, tags, parent):
        lock = self.config / 'swarm/catalog.lock'
        with lock.open('r+') as held:
            fcntl.flock(held, fcntl.LOCK_EX)
            start = time.monotonic()
            with ThreadPoolExecutor(max_workers=2) as pool:
                launches = [pool.submit(self.launch, tag, parent) for tag in tags]
                names = [future.result(timeout=30) for future in launches]
            # Both supervisors have passed their three-second placement cap.
            # Keep the actual catalog lock continuously held for longer still.
            while time.monotonic() - start < 4:
                time.sleep(.02)
            self.assertTrue(self.free(self.state / '.lock'))
            elapsed = time.monotonic() - start
            fcntl.flock(held, fcntl.LOCK_UN)
        self.assertLess(elapsed, 15, 'catalog contention blocked native launch')
        facts = [dict(name=name, session_id=self.binding(name)['session_id'],
                      run_id=self.binding(name)['run_id'],
                      parent=self.binding(name).get('parent'),
                      project=self.project_of(name), explicit=self.explicit(name)) for name in names]
        print('CONTENTION', json.dumps(dict(held_seconds=elapsed, children=facts)), flush=True)
        self.assertTrue(all(not row['explicit'] for row in facts))
        self.assertEqual(len({row['session_id'] for row in facts}), len(names))
        return names

    def eventual(self, names, project):
        process = self.worker()
        def placed():
            self.assertIsNone(process.poll(), (self.root/'catalog-worker.log').read_text())
            return all(self.project_of(name) == project for name in names)
        self.until(placed, 'new launch placement was lost after the catalog became available', within=12)

    def move(self, name, project):
        before = self.swarm('get')
        desired = json.loads(json.dumps(before['organization']))
        token = self.node + '\n' + name
        for row in desired['projects']:
            row['sessions'] = [value for value in row['sessions'] if value != token]
            if row['id'] == project:
                row['sessions'].append(token)
        self.swarm('apply', request=dict(base=before['organization'], desired=desired,
                                        versions=before['versions']))
        self.assertEqual(self.project_of(name), project)

    def test_concurrent_siblings_are_eventually_placed_after_catalog_contention(self):
        parents = self.catalog()
        names = self.contended(['sibling-one', 'sibling-two'], parents['work'])
        self.eventual(names, 'work')
        self.assertEqual(self.project_of(names[0]), 'work')
        self.assertEqual(self.project_of(names[1]), 'work')

    def test_a_person_move_while_placement_is_pending_wins(self):
        parents = self.catalog()
        moved, sibling = self.contended(['person-moved', 'pending-sibling'], parents['work'])
        self.move(moved, 'other')  # Actual project mutation writer, not a forged row.
        self.eventual([sibling], 'work')
        self.assertEqual(self.project_of(moved), 'other')

    def test_a_new_sessions_folder_inference_survives_catalog_contention(self):
        self.catalog()
        names = self.contended(['folder-inferred'], None)
        self.eventual(names, 'tree')
