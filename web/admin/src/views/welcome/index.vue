<script setup lang="ts">
defineOptions({
  name: "Welcome"
});
</script>

<template>
  <div class="welcome-container">
    <!-- Hero Section - Big Typography -->
    <div class="hero-section animate-fade-in-up">
      <div class="new-label">New</div>
      <h1 class="hero-title">EMS Pro</h1>
      <p class="hero-subtitle">
        Master your energy management.<br/>
        Effortlessly.
      </p>
      
      <div class="hero-actions">
        <el-button type="primary" size="large" round class="cta-button">
          Get Started
        </el-button>
        <el-button link class="learn-more">
          Learn more <span class="arrow">›</span>
        </el-button>
      </div>
    </div>

    <!-- Stats Grid - Minimalist Cards -->
    <div class="section-grid animate-fade-in-up animate-delay-100">
      <div class="bento-card card-large">
        <div class="card-content">
          <div class="card-label">Real-time</div>
          <div class="card-value">0ms</div>
          <p class="card-desc">Latency monitoring.</p>
        </div>
        <div class="card-visual visual-graph"></div>
      </div>

      <div class="bento-card">
        <div class="card-content">
          <div class="card-label">Security</div>
          <div class="card-icon-wrapper security-icon">
            <svg viewBox="0 0 24 24" fill="currentColor" width="40" height="40">
              <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm0 10.99h7c-.53 4.12-3.28 7.79-7 8.94V12H5V6.3l7-3.11v8.8z"/>
            </svg>
          </div>
          <p class="card-desc">Enterprise-grade encryption.</p>
        </div>
      </div>

      <div class="bento-card">
        <div class="card-content">
          <div class="card-label">Analytics</div>
          <div class="card-icon-wrapper analytics-icon">
             <svg viewBox="0 0 24 24" fill="currentColor" width="40" height="40">
              <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zM9 17H7v-7h2v7zm4 0h-2V7h2v10zm4 0h-2v-4h2v4z"/>
            </svg>
          </div>
          <p class="card-desc">Deep data insights.</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.welcome-container {
  max-width: 980px; /* Apple classic width */
  margin: 0 auto;
  padding: 60px 20px;
  font-family: var(--font-display);
  color: var(--color-gray-900);
}

/* Hero Section */
.hero-section {
  text-align: center;
  margin-bottom: 100px;
}

.new-label {
  color: var(--color-orange);
  font-size: 12px;
  font-weight: 600;
  margin-bottom: 10px;
  letter-spacing: 0.02em;
}

.hero-title {
  font-size: 80px; /* Massive typography */
  font-weight: 600;
  line-height: 1.05;
  letter-spacing: -0.015em;
  margin-bottom: 10px;
  background: linear-gradient(180deg, #1d1d1f 0%, #424245 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.hero-subtitle {
  font-size: 28px;
  font-weight: 400;
  line-height: 1.2;
  color: var(--color-gray-900);
  margin-bottom: 30px;
}

.hero-actions {
  display: flex;
  justify-content: center;
  gap: 20px;
  align-items: center;
}

.cta-button {
  background-color: var(--color-blue);
  font-size: 17px;
  padding: 12px 26px;
  height: auto;
}

.learn-more {
  font-size: 19px;
  color: var(--color-blue);
  text-decoration: none;
  
  &:hover {
    text-decoration: underline;
  }
  
  .arrow {
    font-size: 20px;
    margin-left: 2px;
  }
}

/* Grid Layout - Bento Box Style */
.section-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 20px;
}

.bento-card {
  background: #f5f5f7; /* Apple light gray bg */
  border-radius: 28px;
  padding: 30px;
  height: 300px;
  position: relative;
  overflow: hidden;
  contain: content; /* Optimize burden: Containment */
  transition: transform 0.3s cubic-bezier(0.25, 0.1, 0.25, 1);
  
  &:hover {
    transform: scale(1.02);
  }
}

.card-large {
  grid-column: span 2;
  background: #fff;
  border: 1px solid var(--color-gray-100);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-content {
  position: relative;
  z-index: 2;
}

.card-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-gray-500);
  text-transform: uppercase;
  margin-bottom: 8px;
}

.card-value {
  font-size: 48px;
  font-weight: 700;
  color: var(--color-gray-900);
  margin-bottom: 8px;
}

.card-desc {
  font-size: 17px;
  font-weight: 600;
  color: var(--color-gray-500);
  margin: 0;
}

.card-icon-wrapper {
  margin: 20px 0;
  color: var(--color-gray-900);
}

.security-icon {
  color: var(--color-blue);
}

.analytics-icon {
  color: var(--color-purple);
}

/* Dark mode adaptation */
:deep(.dark) .welcome-container {
  color: #fff;
}

:deep(.dark) .hero-title {
  background: linear-gradient(180deg, #fff 0%, #a1a1a6 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

:deep(.dark) .hero-subtitle {
  color: #f5f5f7;
}

:deep(.dark) .bento-card {
  background: #1c1c1e;
}

:deep(.dark) .card-large {
  background: #000;
  border-color: #333;
}

:deep(.dark) .card-value {
  color: #fff;
}

/* Responsive */
@media (max-width: 768px) {
  .hero-title {
    font-size: 48px;
  }
  
  .section-grid {
    grid-template-columns: 1fr;
  }
  
  .card-large {
    grid-column: span 1;
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
