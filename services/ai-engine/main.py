from fastapi import FastAPI, HTTPException, Request
from pydantic import BaseModel
from typing import List, Optional
import numpy as np
import torch
from sentence_transformers import SentenceTransformer
from sklearn.ensemble import IsolationForest
import uvicorn
import os
import json

app = FastAPI(title="Veloxx AI Engine")

# Load embedding model (all-MiniLM-L6-v2) for log semantic search.
# 384 dimensions.
model_name = "sentence-transformers/all-MiniLM-L6-v2"
embedding_model = SentenceTransformer(model_name)

# ---------------------------------------------------------------------------
# Models
# ---------------------------------------------------------------------------

class DetectRequest(BaseModel):
    metric_name: str
    values: List[float]
    tenant_id: str

class DetectResponse(BaseModel):
    anomaly_score: float
    is_anomaly: bool
    reason: Optional[str] = None

class EmbedRequest(BaseModel):
    text: str

class EmbedResponse(BaseModel):
    vector: List[float]

class ExplainRequest(BaseModel):
    service: str
    question: str
    metric_data: Optional[dict] = None
    recent_logs: Optional[List[str]] = None

class ExplainResponse(BaseModel):
    explanation: str

# ---------------------------------------------------------------------------
# Endpoints
# ---------------------------------------------------------------------------

@app.post("/detect", response_model=DetectResponse)
async def detect_anomalies(req: DetectRequest):
    if len(req.values) < 5:
        return DetectResponse(anomaly_score=0.0, is_anomaly=False, reason="Insufficient history")

    # Multivariate / univariate Isolation Forest check.
    X = np.array(req.values).reshape(-1, 1)
    # n_estimators=100 behaves as baseline; contamination is proportion of anomalies.
    clf = IsolationForest(n_estimators=100, contamination=0.1, random_state=42)
    clf.fit(X)

    # Prediction: 1 = normal, -1 = anomaly.
    # Scores: high = normal, low = anomaly.
    preds = clf.predict(X)
    scores = clf.decision_function(X)

    # Check last point.
    is_anomaly = bool(preds[-1] == -1)
    # Map score to [0,1] range where 1 is high anomaly.
    # Typically scores range between [-0.5, 0.5]
    score = float(1.0 - (scores[-1] + 0.5))

    return DetectResponse(anomaly_score=score, is_anomaly=is_anomaly)

@app.post("/embed", response_model=EmbedResponse)
async def embed_text(req: EmbedRequest):
    # Perform embedding.
    embedding = embedding_model.encode(req.text)
    return EmbedResponse(vector=embedding.tolist())

@app.post("/explain", response_model=ExplainResponse)
async def explain_anomaly(req: ExplainRequest):
    # Simulated LLM root-cause layer.
    # In production, this calls Anthropic Claude or similar.
    # We use a rule-based stub here to satisfy the architectural requirement.
    expl = f"Root cause analysis for {req.service}: "
    if req.recent_logs:
        if any("timeout" in l.lower() for l in req.recent_logs):
            expl += "Dependency timeout detected in downstream service."
        elif any("error" in l.lower() for l in req.recent_logs):
            expl += "Application level exceptions found in recent logs."
        else:
            expl += "Unusual log patterns detected; suggest manual audit."
    else:
        expl += "Insufficient context to provide automated root cause."

    return ExplainResponse(explanation=expl)

@app.get("/health")
async def health():
    return {"status": "ok"}

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
