package com.whispercppdemo.model

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL

/**
 * Downloads the Whisper Small model (~466 MB) on first launch.
 * Model hosted on HuggingFace: ggerganov/whisper.cpp
 */
object ModelDownloader {
    private const val TAG = "ModelDownloader"
    private const val MODEL_URL = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
    private const val MODEL_FILENAME = "ggml-small.bin"

    /**
     * Returns the model file, downloading it if necessary.
     * @param context Application context for filesDir access
     * @param onProgress Called with download percentage (0-100)
     */
    suspend fun getModel(
        context: Context,
        onProgress: (Int) -> Unit = {}
    ): File = withContext(Dispatchers.IO) {
        val modelsDir = File(context.filesDir, "models").apply { mkdirs() }
        val modelFile = File(modelsDir, MODEL_FILENAME)

        if (modelFile.exists() && modelFile.length() > 100_000_000) {
            Log.d(TAG, "Model already downloaded: ${modelFile.length() / 1_048_576} MB")
            onProgress(100)
            return@withContext modelFile
        }

        Log.i(TAG, "Downloading model from $MODEL_URL")
        val url = URL(MODEL_URL)
        val connection = url.openConnection() as HttpURLConnection
        connection.connectTimeout = 15_000
        connection.readTimeout = 600_000

        val contentLength = connection.contentLength
        Log.i(TAG, "Model size: ${contentLength / 1_048_576} MB")

        val tempFile = File(modelsDir, "$MODEL_FILENAME.tmp")
        connection.inputStream.use { input ->
            FileOutputStream(tempFile).use { output ->
                val buffer = ByteArray(8192)
                var bytesRead: Int
                var totalRead = 0L

                while (input.read(buffer).also { bytesRead = it } != -1) {
                    output.write(buffer, 0, bytesRead)
                    totalRead += bytesRead
                    if (contentLength > 0) {
                        val progress = (totalRead * 100 / contentLength).toInt()
                        if (progress % 10 == 0) onProgress(progress)
                    }
                }
            }
        }

        tempFile.renameTo(modelFile)
        Log.i(TAG, "Model downloaded: ${modelFile.length() / 1_048_576} MB")
        onProgress(100)
        modelFile
    }
}
