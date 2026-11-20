import urllib.request, json

run_id = 44698739370
jobs = json.loads(urllib.request.urlopen(f'https://api.github.com/repos/shreyashjagtap157/AetherOS/actions/runs/{run_id}/jobs').read())
for job in jobs.get('jobs', []):
    print(f"Job: {job['name']} - {job['conclusion']}")
    # Get logs URL
    logs_url = job.get('logs_url')
    print(f"  Logs URL: {logs_url}")
    try:
        req = urllib.request.Request(logs_url)
        resp = urllib.request.urlopen(req)
        print(f"  Logs size: {len(resp.read())} bytes")
    except Exception as e:
        print(f"  Logs error: {e}")