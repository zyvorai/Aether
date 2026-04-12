# ML Training Pipeline Example

Complete machine learning pipeline demonstrating GPU-accelerated training, distributed computing, and model deployment using Aether.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Data Ingestion Layer                     │
│                  (Batch Jobs - Kubernetes)                   │
└─────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Data Processing Pipeline                  │
│              (Spark on Kubernetes - 10 workers)              │
└─────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Feature Store (Redis)                     │
│                    100Gi - In-Memory                         │
└─────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                 Distributed Training Cluster                 │
│                    (KubeVirt - 4x V100 GPUs)                │
│               PyTorch Distributed Data Parallel              │
└─────────────────────────────────────────────────────────────┘
          ▼              ▼              ▼              ▼
    ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
    │ Worker 0 │   │ Worker 1 │   │ Worker 2 │   │ Worker 3 │
    │ Master   │   │          │   │          │   │          │
    │ 1x V100  │   │ 1x V100  │   │ 1x V100  │   │ 1x V100  │
    └──────────┘   └──────────┘   └──────────┘   └──────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Model Registry                          │
│                    (MinIO - S3 Compatible)                   │
│                        5Ti Storage                           │
└─────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Model Serving (Kubernetes)                 │
│              TorchServe - 3-10 replicas (HPA)               │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. Data Ingestion
- **Runtime:** Kubernetes (Batch Jobs)
- **Schedule:** Every 6 hours
- **Resources:** 8 CPU, 32Gi memory
- **Purpose:** Fetch and validate training data

### 2. Data Processing
- **Runtime:** Kubernetes
- **Workers:** 10 Spark executors
- **Resources:** 4 CPU, 16Gi per worker
- **Purpose:** Feature engineering, data cleaning

### 3. Feature Store
- **Runtime:** Kubernetes
- **Storage:** 100Gi in-memory
- **Resources:** 8 CPU, 128Gi memory
- **Purpose:** Fast feature access during training

### 4. Training Cluster
- **Runtime:** KubeVirt
- **GPUs:** 4x NVIDIA Tesla V100
- **Resources:** 64 CPU, 512Gi memory per node
- **Framework:** PyTorch with DDP
- **Purpose:** Distributed model training

### 5. Model Registry
- **Runtime:** Kubernetes
- **Storage:** 5Ti object storage
- **Resources:** 4 CPU, 16Gi memory
- **Purpose:** Store trained models and artifacts

### 6. Model Serving
- **Runtime:** Kubernetes
- **Replicas:** 3-10 (auto-scaling)
- **Resources:** 2 CPU, 8Gi per replica
- **Framework:** TorchServe
- **Purpose:** Production inference API

### 7. Experiment Tracking
- **Tool:** MLflow
- **Runtime:** Kubernetes
- **Storage:** PostgreSQL + S3
- **Purpose:** Track experiments, metrics, parameters

### 8. Monitoring
- **Runtime:** Kubernetes
- **Stack:** Prometheus + Grafana
- **Purpose:** Monitor training, GPU utilization

## Deployment

### Prerequisites

```bash
# GPU-enabled Kubernetes cluster required
kubectl get nodes -o json | jq '.items[].status.capacity."nvidia.com/gpu"'

# Install NVIDIA device plugin
kubectl apply -f https://raw.githubusercontent.com/NVIDIA/k8s-device-plugin/main/nvidia-device-plugin.yml

# Create namespace
kubectl create namespace ml-pipeline
```

### Deploy Infrastructure

```bash
# 1. Deploy feature store (Redis)
aether -s infrastructure/redis.yaml run

# 2. Deploy model registry (MinIO)
aether -s infrastructure/minio.yaml run

# 3. Deploy experiment tracking (MLflow)
aether -s infrastructure/mlflow.yaml run

# 4. Deploy monitoring
kubectl apply -f monitoring/prometheus.yaml
kubectl apply -f monitoring/grafana.yaml
```

### Deploy Pipeline Components

```bash
# 1. Data ingestion job
aether -s pipeline/data-ingestion.yaml run

# 2. Data processing (Spark)
aether -s pipeline/spark-processing.yaml run

# 3. Training cluster
aether -s training/bert-training.yaml run

# 4. Model serving
aether -s serving/torchserve.yaml run
```

## Training Workflows

### Single-GPU Training

```bash
# Train on single GPU
aether -s training/single-gpu-training.yaml run

# Monitor progress
aether logs bert-training --follow

# Access TensorBoard
kubectl port-forward -n ml-pipeline svc/tensorboard 6006:6006
# Open http://localhost:6006
```

### Distributed Training (4 GPUs)

```bash
# Deploy distributed training
aether -s training/distributed-training.yaml run

# Monitor all workers
kubectl logs -n ml-pipeline -l job=training --all-containers=true -f

# Check GPU utilization
kubectl exec -n ml-pipeline bert-training-master-0 -- nvidia-smi
```

### Hyperparameter Tuning

```bash
# Launch Optuna study
aether -s training/hyperparameter-search.yaml run

# Monitor study progress
kubectl logs -n ml-pipeline -l app=optuna-study -f

# View best parameters
kubectl exec -it optuna-master-0 -- \
  python -c "import optuna; study = optuna.load_study(...); print(study.best_params)"
```

## Data Pipeline

### Data Ingestion

```bash
# Ingest from S3
kubectl apply -f pipeline/data-ingestion-s3.yaml

# Ingest from database
kubectl apply -f pipeline/data-ingestion-postgres.yaml

# Schedule regular ingestion
kubectl apply -f pipeline/data-ingestion-cron.yaml
```

### Feature Engineering

```bash
# Run Spark job
aether -s pipeline/feature-engineering.yaml run

# Monitor Spark UI
kubectl port-forward -n ml-pipeline svc/spark-ui 4040:4040
```

### Data Validation

```bash
# Validate data quality
kubectl apply -f pipeline/data-validation.yaml

# Check validation results
kubectl logs -n ml-pipeline -l app=data-validation
```

## Model Registry

### Save Model

```python
import mlflow

# Start MLflow run
with mlflow.start_run():
    # Train model
    model = train_model()

    # Log metrics
    mlflow.log_metric("accuracy", 0.95)
    mlflow.log_metric("loss", 0.15)

    # Log model
    mlflow.pytorch.log_model(model, "model")

    # Tag version
    mlflow.set_tag("version", "v1.2.0")
    mlflow.set_tag("framework", "pytorch")
```

### Load Model

```python
import mlflow

# Load model by run ID
model = mlflow.pytorch.load_model(f"runs:/{run_id}/model")

# Or load by version
model = mlflow.pytorch.load_model("models:/bert-classifier/production")
```

### Promote Model

```bash
# Register model
mlflow models register-model \
  --model-uri runs:/<run-id>/model \
  --name bert-classifier

# Promote to production
mlflow models update-model-version \
  --name bert-classifier \
  --version 2 \
  --stage Production
```

## Model Serving

### Deploy Model

```bash
# Update serving config with new model
yq eval '.env.MODEL_URI = "models:/bert-classifier/production"' \
  -i serving/torchserve.yaml

# Deploy
aether -s serving/torchserve.yaml run

# Wait for ready
kubectl wait --for=condition=ready pod -l app=model-server -n ml-pipeline
```

### Test Inference

```bash
# Get service URL
export MODEL_URL=$(kubectl get svc model-server -n ml-pipeline \
  -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')

# Make prediction
curl -X POST http://$MODEL_URL/predictions/bert \
  -H "Content-Type: application/json" \
  -d '{"text": "This is a test sentence"}'
```

### Auto-Scaling

```bash
# Check HPA status
kubectl get hpa model-server -n ml-pipeline

# Scale based on GPU utilization
kubectl autoscale deployment model-server \
  --cpu-percent=70 \
  --min=3 \
  --max=10 \
  -n ml-pipeline
```

## Monitoring

### GPU Metrics

```bash
# Install DCGM exporter
kubectl apply -f monitoring/dcgm-exporter.yaml

# View GPU metrics
kubectl port-forward -n ml-pipeline svc/dcgm-exporter 9400:9400
curl http://localhost:9400/metrics | grep DCGM
```

### Training Metrics

```bash
# View training dashboard
kubectl port-forward -n ml-pipeline svc/grafana 3000:3000
# Open http://localhost:3000
# Dashboard: ML Training Metrics
```

### Alerts

```yaml
# prometheus-rules.yaml
groups:
  - name: ml-training
    rules:
      - alert: GPUUtilizationLow
        expr: DCGM_FI_DEV_GPU_UTIL < 50
        for: 10m
        annotations:
          summary: "GPU utilization below 50% for 10 minutes"

      - alert: TrainingStalled
        expr: rate(training_steps_total[5m]) == 0
        for: 15m
        annotations:
          summary: "Training has stalled"
```

## Cost Optimization

### Estimate Costs

```bash
# Training cluster cost
aether -s training/distributed-training.yaml cost --provider aws
# Expected: ~$50/hour (4x V100 instances)

# Total pipeline cost
./scripts/calculate-pipeline-cost.sh
# Expected: ~$2,000/month
```

### Spot Instances

```yaml
# Use spot instances for training
nodeSelector:
  node.kubernetes.io/instance-type: p3.2xlarge
  node.kubernetes.io/capacity-type: spot

tolerations:
  - key: nvidia.com/gpu
    operator: Exists
    effect: NoSchedule
```

### Training Optimization

```bash
# Use mixed precision training
--fp16 true

# Gradient accumulation
--gradient-accumulation-steps 4

# Optimize batch size
--per-device-train-batch-size 32
```

## Experiment Tracking

### Log Experiments

```python
import mlflow

mlflow.set_experiment("bert-classifier")

with mlflow.start_run():
    # Log parameters
    mlflow.log_param("learning_rate", 2e-5)
    mlflow.log_param("batch_size", 32)
    mlflow.log_param("num_epochs", 3)

    # Log metrics
    for epoch in range(num_epochs):
        train_loss = train_epoch()
        val_acc = validate()

        mlflow.log_metric("train_loss", train_loss, step=epoch)
        mlflow.log_metric("val_accuracy", val_acc, step=epoch)

    # Log artifacts
    mlflow.log_artifact("model.pth")
    mlflow.log_artifact("config.yaml")
```

### Compare Experiments

```bash
# MLflow UI
kubectl port-forward -n ml-pipeline svc/mlflow 5000:5000
# Open http://localhost:5000

# Compare runs
mlflow ui --backend-store-uri postgresql://mlflow:5432/mlflow
```

## Backup & Recovery

### Checkpoint Strategy

```python
# Save checkpoints every N steps
if step % checkpoint_interval == 0:
    torch.save({
        'epoch': epoch,
        'model_state_dict': model.state_dict(),
        'optimizer_state_dict': optimizer.state_dict(),
        'loss': loss,
    }, f'checkpoint-{step}.pt')

    # Upload to S3
    s3.upload_file(
        f'checkpoint-{step}.pt',
        'my-bucket',
        f'checkpoints/run-{run_id}/checkpoint-{step}.pt'
    )
```

### Resume Training

```python
# Load checkpoint
checkpoint = torch.load('checkpoint-5000.pt')
model.load_state_dict(checkpoint['model_state_dict'])
optimizer.load_state_dict(checkpoint['optimizer_state_dict'])
start_epoch = checkpoint['epoch']
```

## Performance Tuning

### Data Loading

```python
# Optimize DataLoader
train_loader = DataLoader(
    dataset,
    batch_size=32,
    num_workers=8,  # Parallel data loading
    pin_memory=True,  # Faster GPU transfer
    prefetch_factor=2,  # Prefetch batches
)
```

### Mixed Precision Training

```python
from torch.cuda.amp import autocast, GradScaler

scaler = GradScaler()

for batch in train_loader:
    with autocast():
        outputs = model(batch)
        loss = criterion(outputs, labels)

    scaler.scale(loss).backward()
    scaler.step(optimizer)
    scaler.update()
```

### Gradient Checkpointing

```python
# Reduce memory usage
from torch.utils.checkpoint import checkpoint

class Model(nn.Module):
    def forward(self, x):
        return checkpoint(self._forward, x)
```

## CI/CD Integration

### Automated Training

```yaml
# .github/workflows/train.yml
name: Train Model

on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly

jobs:
  train:
    runs-on: ubuntu-latest
    steps:
      - name: Trigger training
        run: |
          aether -s training/weekly-training.yaml run

      - name: Monitor training
        run: |
          ./scripts/monitor-training.sh

      - name: Deploy if improved
        run: |
          if [ $(check-metrics.sh) -eq 0 ]; then
            aether -s serving/torchserve.yaml run
          fi
```

## Troubleshooting

### OOM Errors

```bash
# Reduce batch size
yq eval '.env.BATCH_SIZE = "16"' -i training/config.yaml

# Enable gradient accumulation
yq eval '.env.GRAD_ACCUM_STEPS = "4"' -i training/config.yaml

# Use gradient checkpointing
yq eval '.env.GRADIENT_CHECKPOINTING = "true"' -i training/config.yaml
```

### Slow Training

```bash
# Check GPU utilization
kubectl exec -n ml-pipeline training-pod-0 -- nvidia-smi

# Profile training
kubectl exec -n ml-pipeline training-pod-0 -- \
  python -m torch.utils.bottleneck train.py

# Check data loading
# Should be <10% of total training time
```

### Model Serving Issues

```bash
# Check model server logs
aether logs model-server

# Test health endpoint
curl http://model-server/ping

# Check resource usage
kubectl top pod -n ml-pipeline -l app=model-server
```

## Best Practices

1. **Use checkpoints** - Save model state regularly
2. **Track experiments** - Log all hyperparameters and metrics
3. **Monitor GPUs** - Ensure high utilization (>80%)
4. **Optimize data loading** - Use multiple workers, prefetching
5. **Use mixed precision** - Faster training with less memory
6. **Version models** - Use semantic versioning
7. **Test before deploying** - Validate model performance
8. **Set resource limits** - Prevent runaway jobs
9. **Use spot instances** - Reduce training costs
10. **Backup regularly** - Save checkpoints to object storage

## Example Datasets

- **Image Classification:** ImageNet, CIFAR-10
- **NLP:** GLUE, SQuAD, WikiText
- **Recommendation:** MovieLens, Amazon Reviews
- **Time Series:** Stock prices, Weather data

## Estimated Costs

### Training (4x V100)
- **On-Demand:** $12.24/hour
- **Spot:** ~$3.67/hour (70% savings)
- **Reserved (1yr):** $7.34/hour (40% savings)

### Monthly Pipeline
- **Data Processing:** $200
- **Feature Store:** $300
- **Training (40hrs/month):** $500
- **Model Serving:** $400
- **Storage:** $100
- **Monitoring:** $100
- **Total:** ~$1,600/month

## Documentation

- [Training Guide](docs/training.md)
- [Model Deployment](docs/deployment.md)
- [Performance Tuning](docs/performance.md)
- [Cost Optimization](docs/cost-optimization.md)

## Support

For ML pipeline questions:
- GitHub Issues: https://github.com/ssahani/aether/issues
- Tag: `example-ml-pipeline`
