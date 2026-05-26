package com.whispercppdemo.llm

import android.content.res.AssetManager
import android.util.Log

/**
 * On-device LLM inference using Qwen2.5-0.5B via llama.cpp.
 * Model: qwen2.5-0.5b-instruct-q4_k_m.gguf (~400 MB, INT4 quantized)
 *
 * Note: Requires llama.cpp JNI bridge compiled as native library.
 * For now, loads the model and provides the inference interface.
 */
object LlamaInference {
    private const val TAG = "LlamaInference"
    private const val MODEL_PATH = "models/qwen2.5-0.5b-instruct-q4_k_m.gguf"

    private var nativeLoaded = false
    private var modelPtr: Long = 0

    /**
     * Initialize the LLM. Call once on app startup.
     */
    suspend fun initialize(assets: AssetManager): Boolean {
        return try {
            if (!loadNativeLibrary()) {
                Log.w(TAG, "Native library not available — LLM disabled")
                return false
            }
            Log.i(TAG, "Loading Qwen2.5-0.5B from assets...")
            modelPtr = nativeLoadModel(assets, MODEL_PATH)
            if (modelPtr != 0L) {
                Log.i(TAG, "Model loaded successfully")
                true
            } else {
                Log.e(TAG, "Failed to load model")
                false
            }
        } catch (e: Exception) {
            Log.e(TAG, "LLM initialization failed", e)
            false
        }
    }

    /**
     * Generate a summary of the transcribed text.
     * @param transcript The full transcription text
     * @param onToken Called for each generated token (for streaming display)
     * @return The generated summary
     */
    suspend fun summarize(
        transcript: String,
        onToken: ((String) -> Unit)? = null
    ): String {
        if (modelPtr == 0L) return "LLM not available"

        val prompt = buildString {
            append("<|im_start|>system\n")
            append("You are a helpful assistant. Summarize the following transcript in 2-3 concise sentences. Keep the key points and main message. Respond in English.\n")
            append("<|im_end|>\n")
            append("<|im_start|>user\n")
            append(transcript.take(1500)) // Limit input to ~1500 chars for 0.5B model
            append("\n<|im_end|>\n")
            append("<|im_start|>assistant\n")
        }

        Log.d(TAG, "Prompt length: ${prompt.length} chars")
        return nativeGenerate(modelPtr, prompt) ?: "Summary generation failed"
    }

    /**
     * Release the model. Call in onCleared().
     */
    fun release() {
        if (modelPtr != 0L) {
            nativeFreeModel(modelPtr)
            modelPtr = 0
        }
    }

    private fun loadNativeLibrary(): Boolean {
        if (nativeLoaded) return true
        return try {
            System.loadLibrary("llama_android")
            nativeLoaded = true
            true
        } catch (e: UnsatisfiedLinkError) {
            Log.w(TAG, "llama_android native library not found: ${e.message}")
            false
        }
    }

    // Native methods — implemented in llama_jni.c
    private external fun nativeLoadModel(assets: AssetManager, path: String): Long
    private external fun nativeGenerate(modelPtr: Long, prompt: String): String
    private external fun nativeFreeModel(modelPtr: Long)
}
