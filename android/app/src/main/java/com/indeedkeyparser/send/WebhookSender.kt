package com.indeedkeyparser.send

import android.content.Context
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import androidx.work.workDataOf
import com.indeedkeyparser.parse.Entry
import com.indeedkeyparser.settings.Settings
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.time.Instant
import java.util.concurrent.TimeUnit

fun buildPayload(entry: Entry, timestamp: String): String =
    JSONObject()
        .put("account", entry.account)
        .put("code", entry.code)
        .put("timestamp", timestamp)
        .put("source", "com.indeedid.key")
        .toString()

object WebhookSender {
    fun enqueue(context: Context, entry: Entry) {
        val data = workDataOf(
            "account" to entry.account,
            "code" to entry.code,
            "timestamp" to Instant.now().toString(),
        )
        // DIRECT mode posts over loopback (adb reverse) and needs no network, so
        // requiring one would stall delivery on a phone with no connection.
        val networkType =
            if (Settings(context).requiresNetwork) NetworkType.CONNECTED else NetworkType.NOT_REQUIRED
        val constraints = Constraints.Builder()
            .setRequiredNetworkType(networkType)
            .build()
        val req = OneTimeWorkRequestBuilder<SendCodeWorker>()
            .setInputData(data)
            .setConstraints(constraints)
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 10, TimeUnit.SECONDS)
            .build()
        WorkManager.getInstance(context).enqueue(req)
    }
}

class SendCodeWorker(ctx: Context, params: WorkerParameters) : Worker(ctx, params) {
    override fun doWork(): Result {
        val settings = Settings(applicationContext)
        val url = settings.resolvedUrl
        if (url.isEmpty()) return Result.failure()
        val entry = Entry(
            inputData.getString("account")!!,
            inputData.getString("code")!!,
        )
        val ts = inputData.getString("timestamp")!!
        val body = buildPayload(entry, ts).toRequestBody("application/json".toMediaType())
        val request = Request.Builder()
            .url(url)
            .header("Authorization", "Bearer ${settings.secret}")
            .post(body)
            .build()
        return try {
            OkHttpClient().newCall(request).execute().use { resp ->
                when {
                    resp.isSuccessful -> Result.success()
                    resp.code in 500..599 -> Result.retry()
                    else -> Result.failure()
                }
            }
        } catch (e: Exception) {
            Result.retry()
        }
    }
}
