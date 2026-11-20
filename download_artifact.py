import urllib.request, json, zipfile, io

# Get the latest run
runs = json.loads(urllib.request.urlopen('https://api.github.com/repos/shreyashjagtap157/AetherOS/actions/runs').read())
run_id = None
for r in runs.get('workflow_runs', []):
    if 'RISC-V64' in r['name']:
        run_id = r['id']
        break

print(f"Run ID: {run_id}")

# Get artifacts
artifacts_url = f'https://api.github.com/repos/shreyashjagtap157/AetherOS/actions/runs/{run_id}/artifacts'
artifacts = json.loads(urllib.request.urlopen(artifacts_url).read())
for a in artifacts.get('artifacts', []):
    print(f"Artifact: {a['name']} (id={a['id']})")
    # Download
    download_url = a['archive_download_url']
    print(f"  Download URL: {download_url}")
    # Try to download
    try:
        req = urllib.request.Request(download_url)
        resp = urllib.request.urlopen(req)
        data = resp.read()
        print(f"  Downloaded {len(data)} bytes")
        # Extract
        z = zipfile.ZipFile(io.BytesIO(data))
        for name in z.namelist():
            print(f"  File: {name}")
            if 'boot.log' in name:
                content = z.read(name).decode('utf-8', errors='replace')
                print("=== BOOT LOG ===")
                print(content[:2000])
                print("=== END ===")
    except Exception as e:
        print(f"  Error: {e}")