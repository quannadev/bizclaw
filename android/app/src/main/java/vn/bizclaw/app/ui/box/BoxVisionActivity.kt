package vn.bizclaw.app.ui.box

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.BitmapFactory
import android.os.Bundle
import android.provider.MediaStore
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.camera.core.*
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.core.app.ActivityCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import vn.bizclaw.app.R
import vn.bizclaw.app.box.*
import java.io.File
import java.util.concurrent.ExecutorService

/**
 * Box Vision Activity - Camera AI analysis
 */
class BoxVisionActivity : AppCompatActivity() {
    
    private lateinit var boxEngine: BoxEngine
    private lateinit var cameraProvider: ProcessCameraProvider
    
    private lateinit var imageCapture: ImageCapture
    private lateinit var cameraExecutor: ExecutorService
    
    private lateinit var captureButton: Button
    private lateinit var resultCard: CardView
    private lateinit var imagePreview: ImageView
    private lateinit var analysisResult: TextView
    private lateinit var objectsList: TextView
    
    private var lastImagePath: String? = null
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_box_vision)
        
        initViews()
        initCamera()
        initBoxEngine()
    }
    
    private fun initViews() {
        captureButton = findViewById(R.id.captureButton)
        resultCard = findViewById(R.id.resultCard)
        imagePreview = findViewById(R.id.imagePreview)
        analysisResult = findViewById(R.id.analysisResult)
        objectsList = findViewById(R.id.objectsList)
        
        captureButton.setOnClickListener { captureImage() }
        
        resultCard.visibility = View.GONE
        
        findViewById<Button>(R.id.galleryButton).setOnClickListener {
            openGallery()
        }
    }
    
    private fun initCamera() {
        if (ActivityCompat.checkSelfPermission(this, Manifest.permission.CAMERA) 
            != PackageManager.PERMISSION_GRANTED) {
            ActivityCompat.requestPermissions(
                this,
                arrayOf(Manifest.permission.CAMERA),
                1002
            )
            return
        }
        
        val cameraProviderFuture = ProcessCameraProvider.getInstance(this)
        cameraProviderFuture.addListener({
            cameraProvider = cameraProviderFuture.get()
            bindCameraUseCases()
        }, getMainExecutor())
    }
    
    private fun bindCameraUseCases() {
        val cameraProvider = cameraProvider ?: return
        
        imageCapture = ImageCapture.Builder()
            .setCaptureMode(ImageCapture.CAPTURE_MODE_MINIMIZE_LATENCY)
            .build()
        
        val cameraSelector = CameraSelector.DEFAULT_BACK_CAMERA
        
        try {
            cameraProvider.unbindAll()
            cameraProvider.bindToLifecycle(
                this, cameraSelector, imageCapture
            )
        } catch (e: Exception) {
            Toast.makeText(this, "Camera error: ${e.message}", Toast.LENGTH_SHORT).show()
        }
    }
    
    private fun initBoxEngine() {
        boxEngine = BoxEngine(this)
        
        lifecycleScope.launch {
            val config = BoxConfig(
                modelPath = getExternalFilesDir("models")?.absolutePath + "/hermes-2-pro-q4.gguf",
                visionEnabled = true
            )
            
            boxEngine.initialize(config)
        }
    }
    
    private fun captureImage() {
        val imageCapture = imageCapture ?: return
        
        val photoFile = File(
            getExternalFilesDir(null),
            "box_vision_${System.currentTimeMillis()}.jpg"
        )
        
        val outputOptions = ImageCapture.OutputFileOptions.Builder(photoFile).build()
        
        imageCapture.takePicture(
            outputOptions,
            getMainExecutor(),
            object : ImageCapture.OnImageSavedCallback {
                override fun onImageSaved(output: ImageCapture.OutputFileResults) {
                    lastImagePath = photoFile.absolutePath
                    analyzeImage(photoFile.absolutePath)
                }
                
                override fun onError(exception: ImageCaptureException) {
                    runOnUiThread {
                        Toast.makeText(
                            this@BoxVisionActivity,
                            "Capture failed: ${exception.message}",
                            Toast.LENGTH_SHORT
                        ).show()
                    }
                }
            }
        )
    }
    
    private fun analyzeImage(imagePath: String) {
        lifecycleScope.launch {
            val bitmap = BitmapFactory.decodeFile(imagePath)
            runOnUiThread {
                imagePreview.setImageBitmap(bitmap)
            }
            
            val result = boxEngine.analyzeVision(imagePath)
            
            runOnUiThread {
                resultCard.visibility = View.VISIBLE
                
                result.onSuccess { visionResult ->
                    analysisResult.text = visionResult.description
                    
                    if (visionResult.objects.isNotEmpty()) {
                        objectsList.text = visionResult.objects.joinToString("\n") { obj ->
                            "• ${obj.label} (${(obj.confidence * 100).toInt()}%)"
                        }
                    }
                }.onFailure { e ->
                    analysisResult.text = "Error: ${e.message}"
                }
            }
        }
    }
    
    private fun openGallery() {
        val intent = Intent(Intent.ACTION_PICK, MediaStore.Images.Media.EXTERNAL_CONTENT_URI)
        startActivityForResult(intent, 1003)
    }
    
    @Deprecated("Deprecated in API")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == 1003 && data != null) {
            val uri = data.data ?: return
            val inputStream = contentResolver.openInputStream(uri)
            val file = File(cacheDir, "temp_gallery_${System.currentTimeMillis()}.jpg")
            inputStream?.use { input ->
                file.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
            lastImagePath = file.absolutePath
            analyzeImage(file.absolutePath)
        }
    }
    
    override fun onDestroy() {
        super.onDestroy()
        boxEngine.release()
    }
}
