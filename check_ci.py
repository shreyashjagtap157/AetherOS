import urllib.request, json
run_id = 44698052650
data = json.loads(urllib.request.urlopen(f'https://api.github.com/repos/shreyashjagtap157/AetherOS/actions/runs/{run_id}/jobs').read())
for job in data.get('jobs', []):
    print(f"Job: {job['name']} - {job['conclusion']}")
    for step in job.get('steps', []):
        print(f"  Step: {step['name']} - {step['conclusion']}")