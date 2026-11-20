---
hero:
  eyebrow: GUIDES
  title: Backup and Restore Guide
  tone: violet
---

Aether provides comprehensive backup and restore functionality for workload state management.

## Overview

The backup system allows you to:
- **Create snapshots** of your workload deployments
- **Restore** previous states
- **Merge** backups with existing state
- **Manage** backup lifecycle
- **Automate** backup cleanup

## Quick Start

### Create a Backup

```bash
# Simple backup (auto-generated name)
aether backup

# Named backup
aether backup -n production-2024-02-06

# Backup with description
aether backup -n weekly-backup -d "Weekly production backup before deploy"
```

### List Backups

```bash
aether list-backups
```

Output:
```
📋 Available backups:

  📄 backup-20240206-143052.json
     Created: 2024-02-06T14:30:52Z
     Workloads: 5
     Version: 0.1.0

  📄 production-2024-02-06.json
     Created: 2024-02-06T15:00:00Z
     Workloads: 8
     Version: 0.1.0
     Description: Weekly production backup

Backup directory: /home/user/.aether/backups
```

### Restore a Backup

```bash
# Replace current state (with confirmation)
aether restore ~/.aether/backups/backup-20240206-143052.json

# Merge with existing state (no overwrite)
aether restore ~/.aether/backups/production-2024-02-06.json --merge
```

## Backup Format

Backups are stored as JSON files with the following structure:

```json
{
  "metadata": {
    "version": "1.0",
    "createdAt": "2024-02-06T14:30:52Z",
    "workloadCount": 5,
    "description": "Weekly production backup",
    "aetherVersion": "0.1.0"
  },
  "workloads": [
    {
      "name": "my-app",
      "runtime": "Kubernetes",
      "instance": {
        "id": "abc123",
        "name": "my-app",
        "runtime": "Kubernetes",
        "image": "ghcr.io/myorg/my-app:latest",
        "createdAt": "2024-02-06T10:00:00Z"
      },
      "specPath": "/path/to/workload.yaml",
      "createdAt": "2024-02-06T10:00:00Z",
      "updatedAt": "2024-02-06T14:00:00Z"
    }
  ]
}
```

## Backup Directory

Default location: `~/.aether/backups/`

### Custom Backup Directory

Set the `AETHER_BACKUP_DIR` environment variable:

```bash
export AETHER_BACKUP_DIR=/mnt/backups/aether
aether backup
```

## Use Cases

### Pre-Migration Backup

Create a backup before performing a migration:

```bash
# Backup current state
aether backup -n pre-migration -d "Before migrating to Kubernetes"

# Perform migration
aether migrate my-app kubernetes

# If something goes wrong, restore
aether restore ~/.aether/backups/pre-migration.json
```

### Disaster Recovery

Regular backups for disaster recovery:

```bash
#!/bin/bash
# backup-script.sh

# Create daily backup
DATE=$(date +%Y%m%d)
aether backup -n "daily-${DATE}" -d "Automated daily backup"

# Keep only last 7 days
find ~/.aether/backups/ -name "daily-*.json" -mtime +7 -delete
```

Add to crontab:
```
0 2 * * * /path/to/backup-script.sh
```

### Environment Promotion

Promote workloads from dev to prod:

```bash
# On dev environment
aether backup -n dev-snapshot -d "Tested workloads ready for prod"

# Transfer backup to prod
scp ~/.aether/backups/dev-snapshot.json prod-server:~/

# On prod environment
aether restore ~/dev-snapshot.json --merge
```

### Version Control Integration

Store backups in version control:

```bash
# Create backup
aether backup -n release-v1.2.0

# Commit to git
cp ~/.aether/backups/release-v1.2.0.json ./backups/
git add backups/release-v1.2.0.json
git commit -m "Backup: release v1.2.0"
git push
```

## Automated Cleanup

### Manual Cleanup

Delete old backups manually:

```bash
# List backups
aether list-backups

# Delete specific backup
rm ~/.aether/backups/old-backup.json
```

### Automated Cleanup Script

```bash
#!/bin/bash
# cleanup-backups.sh

BACKUP_DIR="$HOME/.aether/backups"
KEEP_DAYS=30

echo "Cleaning backups older than ${KEEP_DAYS} days..."

find "${BACKUP_DIR}" -name "*.json" -mtime +${KEEP_DAYS} -exec rm -v {} \;

echo "Cleanup complete"
```

## Best Practices

1. **Regular Backups**
   - Schedule daily backups for production environments
   - Create backups before major changes (migrations, updates)

2. **Naming Convention**
   - Use descriptive names: `prod-weekly-20240206`
   - Include environment: `staging-pre-deploy`
   - Add version info: `release-v1.2.0`

3. **Retention Policy**
   - Daily backups: Keep 7 days
   - Weekly backups: Keep 4 weeks
   - Monthly backups: Keep 12 months

4. **Storage**
   - Store backups on different disk/server
   - Consider cloud storage (S3, Azure Blob, GCS)
   - Encrypt sensitive backups

5. **Testing**
   - Regularly test restore procedures
   - Verify backup integrity
   - Document restore process

6. **Version Compatibility**
   - Backups include Aether version
   - Test compatibility when upgrading
   - Keep backups for version rollback

## Advanced Usage

### Backup Inspection

Inspect backup without restoring:

```bash
# Using jq
cat ~/.aether/backups/backup.json | jq '.metadata'

# View workload names
cat ~/.aether/backups/backup.json | jq '.workloads[].name'

# Count workloads
cat ~/.aether/backups/backup.json | jq '.workloads | length'
```

### Selective Restore

Extract specific workloads from backup:

```python
#!/usr/bin/env python3
import json
import sys

# Load backup
with open(sys.argv[1]) as f:
    backup = json.load(f)

# Filter workloads
workload_name = sys.argv[2]
filtered_workloads = [w for w in backup['workloads'] if w['name'] == workload_name]

# Create new backup with filtered workloads
backup['workloads'] = filtered_workloads
backup['metadata']['workloadCount'] = len(filtered_workloads)
backup['metadata']['description'] = f"Filtered: {workload_name}"

# Save filtered backup
with open('filtered-backup.json', 'w') as f:
    json.dump(backup, f, indent=2)

print(f"Filtered backup saved: filtered-backup.json")
```

Usage:
```bash
python3 filter-backup.py backup.json my-app
aether restore filtered-backup.json
```

### Cloud Storage Integration

#### AWS S3

```bash
#!/bin/bash
# backup-to-s3.sh

# Create backup
aether backup -n "$(date +%Y%m%d-%H%M%S)"

# Upload to S3
aws s3 sync ~/.aether/backups/ s3://my-bucket/aether-backups/

echo "Backup uploaded to S3"
```

#### Azure Blob Storage

```bash
#!/bin/bash
# backup-to-azure.sh

# Create backup
aether backup -n "$(date +%Y%m%d-%H%M%S)"

# Upload to Azure
az storage blob upload-batch \
  --account-name myaccount \
  --destination aether-backups \
  --source ~/.aether/backups/

echo "Backup uploaded to Azure Blob Storage"
```

## Restore Modes

### Full Restore

Replaces entire state:

```bash
aether restore backup.json
```

**Caution:** This will delete all current workload state and replace with backup.

### Merge Restore

Adds missing workloads without overwriting:

```bash
aether restore backup.json --merge
```

**Use Case:**
- Recovering accidentally deleted workloads
- Importing workloads from another environment
- Adding workloads without affecting existing ones

## Troubleshooting

### Backup Fails with "No workloads to backup"

**Cause:** No workloads are currently deployed.

**Solution:**
```bash
aether list  # Verify no workloads exist
```

### Restore Fails with "Failed to parse backup"

**Cause:** Corrupted or incompatible backup file.

**Solution:**
```bash
# Verify JSON syntax
cat backup.json | jq .

# Check backup version
cat backup.json | jq '.metadata.aetherVersion'
```

### Backup Directory Not Found

**Cause:** Backup directory doesn't exist yet.

**Solution:** Directory is automatically created on first backup.

### Permission Denied

**Cause:** Insufficient permissions for backup directory.

**Solution:**
```bash
chmod 755 ~/.aether/backups
```

## Backup Security

### Symlink Protection

Aether protects against path traversal attacks via symlinks in the backup directory:

- **Listing backups:** Symlinks are skipped with a warning
- **Reading backup info:** Symlinks are rejected with an error
- **Deleting backups:** Symlinks are rejected with an error

This prevents an attacker from creating symlinks in the backup directory to read or delete arbitrary files on the system.

### Encryption

Encrypt sensitive backups:

```bash
# Encrypt backup
gpg --encrypt --recipient your-email@example.com backup.json

# Decrypt for restore
gpg --decrypt backup.json.gpg > backup.json
aether restore backup.json
rm backup.json  # Clean up decrypted file
```

### Access Control

Protect backup directory:

```bash
chmod 700 ~/.aether/backups
```

## Monitoring

### Backup Success Tracking

```bash
#!/bin/bash
# monitored-backup.sh

if aether backup -n "daily-$(date +%Y%m%d)"; then
  echo "[SUCCESS] Backup completed at $(date)"
  # Send success notification
  curl -X POST https://monitoring.example.com/backup-success
else
  echo "[FAIL] Backup failed at $(date)"
  # Send alert
  curl -X POST https://monitoring.example.com/backup-failure
  exit 1
fi
```

### Integration with Prometheus

Track backup operations via metrics:

```bash
aether metrics | grep aether_cli_commands_total{command="backup"}
```

## API Integration

For programmatic backup management, use the Rust library:

```rust
use aether::backup::{Backup, BackupManager};
use aether::state::StateStore;

async fn create_backup() -> anyhow::Result<()> {
    let state = StateStore::load(&StateStore::default_path())?;
    let manager = BackupManager::new(BackupManager::default_dir());

    let backup_path = manager.create_backup(
        &state,
        Some("automated-backup".to_string()),
        Some("Created by automated system".to_string())
    )?;

    println!("Backup created: {:?}", backup_path);
    Ok(())
}
```

## Support

For backup-related issues:
- GitHub Issues: https://github.com/zyvorai/Aether/issues
- Tag: `backup`
