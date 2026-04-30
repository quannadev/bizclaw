package vn.bizclaw.app.ui.box

import android.Manifest
import android.content.pm.PackageManager
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.Bundle
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import vn.bizclaw.app.R
import vn.bizclaw.app.box.BoxConfig
import vn.bizclaw.app.box.BoxEngine
import java.io.File

/**
 * Box Voice Activity - Voice chat với AI
 */
class BoxVoiceActivity : AppCompatActivity() {
    
    private lateinit var boxEngine: BoxEngine
    private lateinit var recordButton: Button
    private lateinit var statusText: TextView
    private lateinit var transcriptText: TextView
    private lateinit var voiceWave: VoiceWaveView
    
    private var isRecording = false
    private var audioRecord: AudioRecord? = null
    private val sampleRate = 16000
    private val channelConfig = AudioFormat.CHANNEL_IN_MONO
    private val audioFormat = AudioFormat.ENCODING_PCM_16BIT
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_box_voice)
        
        initViews()
        initBoxEngine()
    }
    
    private fun initViews() {
        recordButton = findViewById(R.id.recordButton)
        statusText = findViewById(R.id.statusText)
        transcriptText = findViewById(R.id.transcriptText)
        voiceWave = findViewById(R.id.voiceWave)
        
        recordButton.setOnClickListener { toggleRecording() }
        
        if (ActivityCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) 
            != PackageManager.PERMISSION_GRANTED) {
            ActivityCompat.requestPermissions(
                this,
                arrayOf(Manifest.permission.RECORD_AUDIO),
                1001
            )
        }
    }
    
    private fun initBoxEngine() {
        boxEngine = BoxEngine(this)
        
        lifecycleScope.launch {
            val config = BoxConfig(
                modelPath = getExternalFilesDir("models")?.absolutePath + "/hermes-2-pro-q4.gguf",
                whisperPath = getExternalFilesDir("models")?.absolutePath + "/whisper-tiny.bin"
            )
            
            boxEngine.initialize(config).onSuccess {
                runOnUiThread {
                    statusText.text = "Ready"
                }
            }
        }
    }
    
    private fun toggleRecording() {
        if (isRecording) {
            stopRecording()
        } else {
            startRecording()
        }
    }
    
    private fun startRecording() {
        if (ActivityCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) 
            != PackageManager.PERMISSION_GRANTED) {
            Toast.makeText(this, "Microphone permission required", Toast.LENGTH_SHORT).show()
            return
        }
        
        val bufferSize = AudioRecord.getMinBufferSize(sampleRate, channelConfig, audioFormat)
        
        audioRecord = AudioRecord(
            MediaRecorder.AudioSource.MIC,
            sampleRate,
            channelConfig,
            audioFormat,
            bufferSize
        )
        
        audioRecord?.startRecording()
        isRecording = true
        
        runOnUiThread {
            recordButton.text = "Stop"
            statusText.text = "Recording..."
            voiceWave.visibility = View.VISIBLE
            voiceWave.startAnimation()
        }
        
        // Start recording thread
        lifecycleScope.launch {
            val audioData = recordAudio()
            processAudio(audioData)
        }
    }
    
    private suspend fun recordAudio(): ByteArray {
        val bufferSize = AudioRecord.getMinBufferSize(sampleRate, channelConfig, audioFormat)
        val buffer = ByteArray(bufferSize * 10) // 10 seconds max
        var totalRead = 0
        
        while (isRecording && totalRead < buffer.size) {
            val read = audioRecord?.read(buffer, totalRead, bufferSize) ?: 0
            if (read > 0) {
                totalRead += read
            }
        }
        
        return buffer.copyOf(totalRead)
    }
    
    private fun stopRecording() {
        isRecording = false
        audioRecord?.stop()
        audioRecord?.release()
        audioRecord = null
        
        runOnUiThread {
            recordButton.text = "Record"
            statusText.text = "Processing..."
            voiceWave.stopAnimation()
        }
    }
    
    private suspend fun processAudio(audioData: ByteArray) {
        val transcription = boxEngine.transcribe(audioData)
        
        runOnUiThread {
            transcription.onSuccess { text ->
                transcriptText.text = text
                statusText.text = "Done"
                
                // Continue to chat with transcribed text
                // navigateToChat(text)
            }.onFailure { e ->
                statusText.text = "Error: ${e.message}"
            }
        }
    }
    
    override fun onDestroy() {
        super.onDestroy()
        if (isRecording) stopRecording()
        boxEngine.release()
    }
}

/**
 * Custom view for voice wave animation
 */
class VoiceWaveView(context: Context?, attrs: AttributeSet?) : View(context, attrs) {
    // Simple wave animation implementation
    fun startAnimation() {
        // Start animating
    }
    
    fun stopAnimation() {
        // Stop animation
    }
}
