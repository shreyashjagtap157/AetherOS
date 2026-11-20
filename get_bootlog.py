import urllib.request, json, re

run_id = 44699992484
jobs = json.loads(urllib.request.urlopen(f'https://api.github.com/repos/shreyashjagtap157/AetherOS/actions/runs/{run_id}/jobs').read())
for job in jobs.get('jobs', []):
    print(f"Job: {job['name']} - {job['conclusion']}")
    # Try to get logs
    try:
        logs = urllib.request.urlopen(job['logs_url'])
        content = logs.read().decode('utf-8', errors='replace')
        # Find boot.log section
        lines = content.split('\n')
        in_bootlog = False
        for line in lines:
            if '=== boot.log ===' in line:
                in_bootlog = True
                continue
            if '=== end boot.log ===' in line:
                in_bootlog = False
                continue
            if in_bootlog:
                # Strip timestamp prefix
                parts = line.split('Z ', 1)
                if len(parts) > 1:
                    print(parts[1])
                else:
                    print(line)
    except Exception as e:
        print(f"  Error: {e}")