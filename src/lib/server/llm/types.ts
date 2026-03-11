export interface CorrectionResult {
	correctedText: string;
}

export interface LLMProvider {
	correctText(text: string): Promise<CorrectionResult>;
}
