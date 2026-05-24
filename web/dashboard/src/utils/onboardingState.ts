const VALIDATED_KEY = 'aether_onboarding_validated';

export function markSpecValidated(): void {
  try {
    localStorage.setItem(VALIDATED_KEY, '1');
  } catch {
    /* ignore */
  }
}

export function hasValidatedSpec(): boolean {
  try {
    return localStorage.getItem(VALIDATED_KEY) === '1';
  } catch {
    return false;
  }
}
