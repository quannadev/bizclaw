// BizClaw Welcome Onboarding — AI-powered first-time setup wizard
// Interviews the user conversationally, fills all Brain Workspace MD files
// Creates a truly personalized AI assistant experience
//
// Flow: Welcome → Chat Interview → Brain Files Generation → Dashboard Ready

const { h, html, useState, useEffect, useCallback, useRef } = window;
import { authFetch } from '/static/dashboard/shared.js';

// ═══ ONBOARDING STEPS ═══
const STEPS = [
  {
    id: 'welcome',
    icon: '👋',
    title: 'Chào mừng đến BizClaw!',
    titleEn: 'Welcome to BizClaw!',
    subtitle: 'Hãy để AI giúp bạn cài đặt trợ lý thông minh riêng',
    subtitleEn: 'Let AI help you set up your personal smart assistant',
  },
  {
    id: 'about_you',
    icon: '🧑‍💼',
    title: 'Giới thiệu về bạn',
    titleEn: 'About You',
    question: 'Hãy giới thiệu ngắn gọn về bạn: bạn tên gì, làm lĩnh vực gì, vai trò gì trong công ty?',
    questionEn: 'Tell me about yourself: your name, field of work, and role?',
    placeholder: 'Ví dụ: Tôi là Hoài, CEO công ty phần mềm, chuyên về AI và automation cho SME',
    brainFile: 'USER.md',
  },
  {
    id: 'business',
    icon: '🏢',
    title: 'Doanh nghiệp của bạn',
    titleEn: 'Your Business',
    question: 'Bạn có thể mô tả doanh nghiệp/dự án hiện tại? Sản phẩm/dịch vụ chính là gì?',
    questionEn: 'Describe your current business/project. What are your main products/services?',
    placeholder: 'Ví dụ: BizClaw cung cấp AI agent cho SME Việt Nam, giúp tự động hóa quy trình kinh doanh',
    brainFile: 'IDENTITY.md',
  },
  {
    id: 'goals',
    icon: '🎯',
    title: 'Mục tiêu sử dụng',
    titleEn: 'Usage Goals',
    question: 'Bạn muốn AI giúp gì? (chăm sóc khách hàng, bán hàng, marketing, nghiên cứu, lập trình, ...?)',
    questionEn: 'What do you want AI to help with? (customer care, sales, marketing, research, coding...?)',
    placeholder: 'Ví dụ: Tự động phản hồi Zalo khi có khách hỏi, tạo nội dung marketing, phân tích dữ liệu',
    brainFile: 'TOOLS.md',
  },
  {
    id: 'personality',
    icon: '✨',
    title: 'Tính cách AI',
    titleEn: 'AI Personality',
    question: 'Bạn muốn AI trợ lý có phong cách như thế nào? (chuyên nghiệp, thân thiện, năng động, sáng tạo...?)',
    questionEn: 'What personality should your AI have? (professional, friendly, energetic, creative...?)',
    placeholder: 'Ví dụ: Chuyên nghiệp nhưng thân thiện, xưng "em" gọi "anh/chị", trả lời ngắn gọn rõ ràng',
    brainFile: 'SOUL.md',
  },
  {
    id: 'channels',
    icon: '📱',
    title: 'Kênh liên lạc',
    titleEn: 'Communication Channels',
    question: 'Bạn muốn kết nối AI với kênh nào? (Zalo, Telegram, Email, Facebook, Website...?)',
    questionEn: 'Which channels do you want to connect? (Zalo, Telegram, Email, Facebook, Website...?)',
    placeholder: 'Ví dụ: Zalo cá nhân, Telegram bot, Email hỗ trợ, tích hợp website',
    brainFile: 'AGENTS.md',
  },
  {
    id: 'security',
    icon: '🔒',
    title: 'Bảo mật & Quyền hạn',
    titleEn: 'Security & Permissions',
    question: 'Bạn có yêu cầu đặc biệt về bảo mật không? (AI không được chia sẻ thông tin nào, giới hạn trả lời chủ đề gì...?)',
    questionEn: 'Any special security requirements? (what info should AI not share, topic limitations...?)',
    placeholder: 'Ví dụ: Không tiết lộ giá gốc, không bàn chính trị, chỉ trả lời trong phạm vi sản phẩm',
    brainFile: 'SECURITY.md',
  },
  {
    id: 'generating',
    icon: '🤖',
    title: 'AI đang tạo hồ sơ...',
    titleEn: 'AI is creating your profile...',
    subtitle: 'Đang phân tích câu trả lời và cấu hình Brain Workspace',
    subtitleEn: 'Analyzing your answers and configuring Brain Workspace',
  },
  {
    id: 'complete',
    icon: '🎉',
    title: 'Hoàn tất!',
    titleEn: 'All Set!',
    subtitle: 'BizClaw đã được cá nhân hóa cho bạn',
    subtitleEn: 'BizClaw has been personalized for you',
  },
];

// ═══ ONBOARDING WIZARD COMPONENT ═══
export function OnboardingWizard({ lang, onComplete }) {
  const [step, setStep] = useState(0);
  const [answers, setAnswers] = useState({});
  const [input, setInput] = useState('');
  const [generating, setGenerating] = useState(false);
  const [generatedFiles, setGeneratedFiles] = useState([]);
  const [error, setError] = useState('');
  const [progress, setProgress] = useState(0);
  const [chatHistory, setChatHistory] = useState([]);
  const chatEndRef = useRef(null);
  const inputRef = useRef(null);
  const isVi = lang !== 'en';

  // Auto-scroll chat
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatHistory, step]);

  // Auto-focus input
  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 300);
  }, [step]);

  // Add AI message to chat
  const addAiMessage = useCallback((text) => {
    setChatHistory(prev => [...prev, { role: 'ai', text, time: Date.now() }]);
  }, []);

  // Add user message to chat
  const addUserMessage = useCallback((text) => {
    setChatHistory(prev => [...prev, { role: 'user', text, time: Date.now() }]);
  }, []);

  // Handle step progression
  const handleNext = useCallback(async () => {
    const currentStep = STEPS[step];

    if (currentStep.id === 'welcome') {
      setStep(1);
      const q = isVi ? STEPS[1].question : STEPS[1].questionEn;
      addAiMessage(q);
      return;
    }

    // Save answer for question steps
    if (currentStep.question && input.trim()) {
      addUserMessage(input.trim());
      setAnswers(prev => ({ ...prev, [currentStep.id]: input.trim() }));
      setInput('');

      const nextIdx = step + 1;
      const nextStep = STEPS[nextIdx];

      if (nextStep && nextStep.question) {
        // Show AI acknowledgment + next question
        const acks = isVi
          ? ['Tuyệt vời! 👍', 'Cảm ơn bạn! 🙏', 'Hay quá! ✨', 'Rất tốt! 💪', 'Đã hiểu! 📝']
          : ['Great! 👍', 'Thanks! 🙏', 'Awesome! ✨', 'Perfect! 💪', 'Got it! 📝'];
        const ack = acks[Math.floor(Math.random() * acks.length)];
        setTimeout(() => {
          addAiMessage(ack);
          setTimeout(() => {
            addAiMessage(isVi ? nextStep.question : nextStep.questionEn);
            setStep(nextIdx);
          }, 500);
        }, 300);
      } else if (nextStep && nextStep.id === 'generating') {
        // Start generation
        setStep(nextIdx);
        setGenerating(true);
        await generateBrainFiles({ ...answers, [currentStep.id]: input.trim() });
      }
      return;
    }

    if (!currentStep.question) {
      setStep(step + 1);
    }
  }, [step, input, answers, isVi, addAiMessage, addUserMessage]);

  // Handle Enter key
  const handleKeyDown = useCallback((e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleNext();
    }
  }, [handleNext]);

  // Generate all Brain Workspace files using AI
  const generateBrainFiles = useCallback(async (allAnswers) => {
    setError('');
    setProgress(10);

    try {
      // Step 1: Generate SOUL.md, IDENTITY.md, USER.md via /api/v1/brain/personalize
      addAiMessage(isVi ? '🧠 Đang phân tích thông tin của bạn...' : '🧠 Analyzing your information...');
      setProgress(20);

      const personalizeRes = await authFetch('/api/v1/brain/personalize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          about_user: `${allAnswers.about_you || ''}. Business: ${allAnswers.business || ''}. Goals: ${allAnswers.goals || ''}`,
          agent_vibe: allAnswers.personality || 'professional and friendly',
          agent_name: 'BizClaw Agent',
          language: isVi ? 'vi' : 'en',
        }),
      });
      const personalizeData = await personalizeRes.json();

      if (personalizeData.ok) {
        setGeneratedFiles(prev => [...prev, ...personalizeData.saved]);
        setProgress(40);
        addAiMessage(isVi ? '✅ Đã tạo hồ sơ AI: SOUL.md, IDENTITY.md, USER.md' : '✅ Created AI profile: SOUL.md, IDENTITY.md, USER.md');
      }

      // Step 2: Generate remaining files manually
      const extraFiles = [
        {
          name: 'MEMORY.md',
          content: generateMemoryMd(allAnswers, isVi),
        },
        {
          name: 'TOOLS.md',
          content: generateToolsMd(allAnswers, isVi),
        },
        {
          name: 'AGENTS.md',
          content: generateAgentsMd(allAnswers, isVi),
        },
        {
          name: 'SECURITY.md',
          content: generateSecurityMd(allAnswers, isVi),
        },
        {
          name: 'BOOT.md',
          content: generateBootMd(allAnswers, isVi),
        },
      ];

      let completed = 0;
      for (const file of extraFiles) {
        try {
          const res = await authFetch(`/api/v1/brain/files/${encodeURIComponent(file.name)}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ content: file.content }),
          });
          const d = await res.json();
          if (d.ok) {
            completed++;
            setGeneratedFiles(prev => [...prev, file.name]);
          }
        } catch (e) {
          console.warn(`Failed to save ${file.name}:`, e);
        }
        setProgress(40 + Math.round((completed / extraFiles.length) * 50));
      }

      setProgress(100);
      addAiMessage(isVi
        ? `🎉 Hoàn tất! Đã tạo ${3 + completed} file Brain Workspace. BizClaw giờ đã hiểu bạn rồi!`
        : `🎉 Done! Created ${3 + completed} Brain Workspace files. BizClaw now understands you!`
      );

      // Mark onboarding complete
      localStorage.setItem('bizclaw_onboarded', 'true');
      localStorage.setItem('bizclaw_onboard_time', new Date().toISOString());

      setTimeout(() => {
        setGenerating(false);
        setStep(STEPS.length - 1); // complete step
      }, 1500);

    } catch (e) {
      setError(e.message || 'Generation failed');
      setGenerating(false);
      addAiMessage(isVi ? '❌ Có lỗi xảy ra. Bạn có thể thử lại hoặc bỏ qua để vào Dashboard.' : '❌ An error occurred. You can retry or skip to Dashboard.');
      setStep(STEPS.length - 1);
    }
  }, [isVi, addAiMessage]);

  // Current step data
  const current = STEPS[step];
  const totalQuestions = STEPS.filter(s => s.question).length;
  const answeredCount = Object.keys(answers).length;
  const progressPct = current.id === 'generating' ? progress : Math.round((step / (STEPS.length - 1)) * 100);

  // ═══ RENDER ═══
  return html`
    <div class="onboarding-overlay">
      <div class="onboarding-container">
        
        ${/* ── PROGRESS BAR ── */null}
        <div class="onboarding-progress">
          <div class="onboarding-progress-bar" style=${{ width: progressPct + '%' }}></div>
        </div>
        <div class="onboarding-progress-text">
          ${current.id === 'generating' ? (isVi ? `Đang tạo... ${progress}%` : `Generating... ${progress}%`) :
            current.id === 'complete' ? (isVi ? '✅ Hoàn tất' : '✅ Complete') :
            `${answeredCount}/${totalQuestions}`}
        </div>

        ${/* ── WELCOME STEP ── */null}
        ${current.id === 'welcome' && html`
          <div class="onboarding-welcome" key="welcome">
            <div class="onboarding-logo">
              <span class="onboarding-logo-icon">⚡</span>
              <span class="onboarding-logo-text">BizClaw</span>
            </div>
            <h1 class="onboarding-title">${isVi ? current.title : current.titleEn}</h1>
            <p class="onboarding-subtitle">${isVi ? current.subtitle : current.subtitleEn}</p>
            
            <div class="onboarding-features">
              <div class="onboarding-feature">
                <span class="feature-icon">🧠</span>
                <div>
                  <strong>${isVi ? 'AI hiểu bạn' : 'AI Understands You'}</strong>
                  <p>${isVi ? 'Phỏng vấn nhanh 2 phút để cá nhân hóa toàn bộ' : '2-minute interview to fully personalize'}</p>
                </div>
              </div>
              <div class="onboarding-feature">
                <span class="feature-icon">📄</span>
                <div>
                  <strong>${isVi ? 'Brain Workspace' : 'Brain Workspace'}</strong>
                  <p>${isVi ? 'Tự động tạo 8 file cấu hình trí tuệ cho AI' : 'Automatically creates 8 intelligence config files'}</p>
                </div>
              </div>
              <div class="onboarding-feature">
                <span class="feature-icon">🚀</span>
                <div>
                  <strong>${isVi ? 'Sẵn sàng ngay' : 'Ready Instantly'}</strong>
                  <p>${isVi ? 'AI trợ lý riêng của bạn, triển khai tức thì' : 'Your personal AI assistant, deployed instantly'}</p>
                </div>
              </div>
            </div>

            <button class="onboarding-btn-primary" onClick=${handleNext}>
              ${isVi ? '🚀 Bắt đầu ngay' : '🚀 Get Started'}
            </button>
            <button class="onboarding-btn-skip" onClick=${() => {
              localStorage.setItem('bizclaw_onboarded', 'true');
              onComplete?.();
            }}>
              ${isVi ? 'Bỏ qua, vào Dashboard →' : 'Skip, go to Dashboard →'}
            </button>
          </div>
        `}

        ${/* ── CHAT INTERVIEW STEPS ── */null}
        ${(current.question || current.id === 'generating') && html`
          <div class="onboarding-chat" key="chat">
            <div class="onboarding-chat-header">
              <span class="onboarding-chat-avatar">🤖</span>
              <div>
                <strong>BizClaw Setup AI</strong>
                <span class="onboarding-chat-status">● ${isVi ? 'Đang trực tuyến' : 'Online'}</span>
              </div>
              <div class="onboarding-step-badge">${current.icon} ${isVi ? current.title : current.titleEn}</div>
            </div>

            <div class="onboarding-chat-messages">
              ${chatHistory.map((msg, i) => html`
                <div key=${i} class=${`onboarding-msg onboarding-msg-${msg.role}`}>
                  ${msg.role === 'ai' && html`<span class="onboarding-msg-avatar">🤖</span>`}
                  <div class="onboarding-msg-bubble">${msg.text}</div>
                  ${msg.role === 'user' && html`<span class="onboarding-msg-avatar">🧑</span>`}
                </div>
              `)}
              ${generating && html`
                <div class="onboarding-msg onboarding-msg-ai">
                  <span class="onboarding-msg-avatar">🤖</span>
                  <div class="onboarding-msg-bubble onboarding-typing">
                    <span></span><span></span><span></span>
                  </div>
                </div>
              `}
              <div ref=${chatEndRef}></div>
            </div>

            ${!generating && current.question && html`
              <div class="onboarding-chat-input">
                <textarea
                  ref=${inputRef}
                  value=${input}
                  onInput=${e => setInput(e.target.value)}
                  onKeyDown=${handleKeyDown}
                  placeholder=${current.placeholder || ''}
                  rows="2"
                  class="onboarding-textarea"
                ></textarea>
                <button
                  class="onboarding-send-btn"
                  onClick=${handleNext}
                  disabled=${!input.trim()}
                >
                  ➤
                </button>
              </div>
            `}

            ${generating && html`
              <div class="onboarding-gen-progress">
                <div class="onboarding-gen-bar" style=${{ width: progress + '%' }}></div>
                <div class="onboarding-gen-files">
                  ${generatedFiles.map(f => html`
                    <span key=${f} class="onboarding-gen-file">✅ ${f}</span>
                  `)}
                </div>
              </div>
            `}
          </div>
        `}

        ${/* ── COMPLETE STEP ── */null}
        ${current.id === 'complete' && html`
          <div class="onboarding-complete" key="complete">
            <div class="onboarding-complete-icon">🎉</div>
            <h1 class="onboarding-title">${isVi ? current.title : current.titleEn}</h1>
            <p class="onboarding-subtitle">${isVi ? current.subtitle : current.subtitleEn}</p>

            <div class="onboarding-files-grid">
              ${generatedFiles.map(f => html`
                <div key=${f} class="onboarding-file-card">
                  <span class="file-icon">📄</span>
                  <span class="file-name">${f}</span>
                  <span class="file-check">✅</span>
                </div>
              `)}
            </div>

            <div class="onboarding-summary">
              <p>${isVi 
                ? '🧠 BizClaw giờ đã hiểu bạn! Tất cả Brain Workspace đã được cấu hình để AI trợ lý hoạt động chính xác theo yêu cầu của bạn.'
                : '🧠 BizClaw now understands you! All Brain Workspace files are configured so your AI assistant works exactly as you need.'
              }</p>
            </div>

            <button class="onboarding-btn-primary" onClick=${() => onComplete?.()}>
              ${isVi ? '🚀 Vào Dashboard' : '🚀 Go to Dashboard'}
            </button>
          </div>
        `}

        ${/* ── ERROR ── */null}
        ${error && html`
          <div class="onboarding-error">
            ⚠️ ${error}
            <button onClick=${() => { setError(''); setStep(STEPS.length - 1); }}>
              ${isVi ? 'Bỏ qua' : 'Skip'}
            </button>
          </div>
        `}
      </div>
    </div>
  `;
}

// ═══ BRAIN FILE GENERATORS ═══

function generateMemoryMd(answers, isVi) {
  const lines = [
    '# MEMORY.md — Long-Term Memory',
    '',
    isVi ? '## Thông tin ghi nhớ dài hạn' : '## Long-term memory notes',
    '',
    isVi ? '### Người dùng' : '### User Info',
    answers.about_you ? `- ${answers.about_you}` : '',
    '',
    isVi ? '### Doanh nghiệp' : '### Business',
    answers.business ? `- ${answers.business}` : '',
    '',
    isVi ? '### Mục tiêu' : '### Goals',
    answers.goals ? `- ${answers.goals}` : '',
    '',
    isVi ? '### Sở thích giao tiếp' : '### Communication Preferences',
    answers.personality ? `- ${answers.personality}` : '',
    '',
    isVi ? '### Kênh ưu tiên' : '### Preferred Channels',
    answers.channels ? `- ${answers.channels}` : '',
    '',
    `> ${isVi ? 'Cập nhật lần cuối' : 'Last updated'}: ${new Date().toISOString().split('T')[0]}`,
  ];
  return lines.filter(l => l !== undefined).join('\n');
}

function generateToolsMd(answers, isVi) {
  const goals = (answers.goals || '').toLowerCase();
  const tools = [];

  if (goals.includes('zalo') || goals.includes('chat') || goals.includes('khách hàng') || goals.includes('customer'))
    tools.push('zalo_tool', 'social_post');
  if (goals.includes('marketing') || goals.includes('nội dung') || goals.includes('content'))
    tools.push('web_search', 'social_post');
  if (goals.includes('lập trình') || goals.includes('code') || goals.includes('dev'))
    tools.push('shell', 'file', 'edit_file', 'glob', 'grep');
  if (goals.includes('dữ liệu') || goals.includes('data') || goals.includes('phân tích') || goals.includes('analys'))
    tools.push('db_query', 'nl_query');
  if (goals.includes('email') || goals.includes('mail'))
    tools.push('http_request');
  if (goals.includes('web') || goals.includes('browser') || goals.includes('scrape'))
    tools.push('browser', 'web_search');

  // Default essential tools
  const allTools = [...new Set(['web_search', 'http_request', ...tools])];

  return [
    '# TOOLS.md — Environment Notes',
    '',
    isVi ? '## Công cụ được bật cho Agent' : '## Enabled tools for Agent',
    '',
    ...allTools.map(t => `- \`${t}\``),
    '',
    isVi ? '## Ghi chú môi trường' : '## Environment Notes',
    '',
    `- Platform: ${navigator.platform || 'Web'}`,
    `- ${isVi ? 'Ngôn ngữ ưu tiên' : 'Preferred language'}: ${isVi ? 'Tiếng Việt' : 'English'}`,
    answers.goals ? `- ${isVi ? 'Mục tiêu chính' : 'Primary goal'}: ${answers.goals}` : '',
    '',
    `> ${isVi ? 'Tự động tạo từ Welcome Onboarding' : 'Auto-generated from Welcome Onboarding'}`,
  ].filter(l => l !== undefined).join('\n');
}

function generateAgentsMd(answers, isVi) {
  const channelsText = (answers.channels || '').toLowerCase();
  const channels = [];

  if (channelsText.includes('zalo')) channels.push({ name: 'Zalo', status: isVi ? 'Cần cấu hình cookie' : 'Needs cookie setup' });
  if (channelsText.includes('telegram')) channels.push({ name: 'Telegram', status: isVi ? 'Cần bot token' : 'Needs bot token' });
  if (channelsText.includes('email')) channels.push({ name: 'Email', status: isVi ? 'Cần IMAP/SMTP' : 'Needs IMAP/SMTP' });
  if (channelsText.includes('facebook') || channelsText.includes('fb')) channels.push({ name: 'Facebook', status: isVi ? 'Qua webhook' : 'Via webhook' });
  if (channelsText.includes('website') || channelsText.includes('web')) channels.push({ name: 'Website Widget', status: isVi ? 'Embed JS' : 'Embed JS' });
  if (channels.length === 0) channels.push({ name: 'Web Dashboard', status: isVi ? 'Sẵn sàng' : 'Ready' });

  return [
    '# AGENTS.md — Workspace Rules',
    '',
    isVi ? '## Quy tắc hoạt động' : '## Operating Rules',
    '',
    isVi ? '### Kênh liên lạc đã đăng ký' : '### Registered Channels',
    ...channels.map(c => `- **${c.name}**: ${c.status}`),
    '',
    isVi ? '### Quy tắc ưu tiên' : '### Priority Rules',
    isVi ? '1. Luôn trả lời trong vòng 30 giây' : '1. Always respond within 30 seconds',
    isVi ? '2. Ưu tiên kênh: ' + channels.map(c => c.name).join(' > ') : '2. Channel priority: ' + channels.map(c => c.name).join(' > '),
    isVi ? '3. Escalate vấn đề phức tạp cho con người' : '3. Escalate complex issues to humans',
    '',
    isVi ? '### Workflow mặc định' : '### Default Workflows',
    isVi ? '- Tin nhắn mới → Phân loại → AI xử lý/Chuyển cho người' : '- New message → Classify → AI handle/Forward to human',
    isVi ? '- Yêu cầu hỗ trợ → Check Knowledge Base → Trả lời' : '- Support request → Check Knowledge Base → Reply',
    '',
    `> ${isVi ? 'Tự động tạo từ Welcome Onboarding' : 'Auto-generated from Welcome Onboarding'}`,
  ].join('\n');
}

function generateSecurityMd(answers, isVi) {
  const securityNotes = answers.security || (isVi ? 'Bảo mật tiêu chuẩn' : 'Standard security');

  return [
    '# SECURITY.md — Security Policies',
    '',
    isVi ? '## Chính sách bảo mật AI' : '## AI Security Policies',
    '',
    isVi ? '### Quy tắc cần tuân thủ' : '### Rules to Follow',
    `- ${securityNotes}`,
    '',
    isVi ? '### Thông tin KHÔNG được chia sẻ' : '### Information NOT to Share',
    isVi ? '- API keys, tokens, mật khẩu' : '- API keys, tokens, passwords',
    isVi ? '- Thông tin tài chính nội bộ' : '- Internal financial information',
    isVi ? '- Dữ liệu khách hàng cá nhân' : '- Personal customer data',
    '',
    isVi ? '### Giới hạn hành vi' : '### Behavioral Limits',
    isVi ? '- Không thực hiện giao dịch tài chính tự động' : '- Do not perform automatic financial transactions',
    isVi ? '- Không gửi tin nhắn hàng loạt khi chưa duyệt' : '- Do not send mass messages without approval',
    isVi ? '- Luôn hỏi lại khi không chắc chắn' : '- Always ask when uncertain',
    '',
    isVi ? '### Mức độ tự chủ' : '### Autonomy Level',
    isVi ? '- Mặc định: Supervised (cần duyệt hành động nhạy cảm)' : '- Default: Supervised (approval needed for sensitive actions)',
    '',
    `> ${isVi ? 'Tự động tạo từ Welcome Onboarding' : 'Auto-generated from Welcome Onboarding'}`,
  ].join('\n');
}

function generateBootMd(answers, isVi) {
  return [
    '# BOOT.md — Startup Checklist',
    '',
    isVi ? '## Checklist khởi động' : '## Startup Checklist',
    '',
    isVi ? '### Khi Agent khởi động, kiểm tra:' : '### When Agent starts, check:',
    isVi ? '- [ ] Đọc SOUL.md → nạp tính cách' : '- [ ] Read SOUL.md → load personality',
    isVi ? '- [ ] Đọc IDENTITY.md → nạp vai trò' : '- [ ] Read IDENTITY.md → load role',
    isVi ? '- [ ] Đọc USER.md → nhớ thông tin người dùng' : '- [ ] Read USER.md → remember user info',
    isVi ? '- [ ] Đọc MEMORY.md → nạp bộ nhớ dài hạn' : '- [ ] Read MEMORY.md → load long-term memory',
    isVi ? '- [ ] Đọc SECURITY.md → áp dụng quy tắc bảo mật' : '- [ ] Read SECURITY.md → apply security rules',
    isVi ? '- [ ] Kiểm tra kết nối kênh liên lạc' : '- [ ] Check channel connections',
    isVi ? '- [ ] Kiểm tra API keys và providers' : '- [ ] Verify API keys and providers',
    '',
    isVi ? '### Log khởi động' : '### Boot Log',
    `- ${isVi ? 'Onboarding hoàn tất' : 'Onboarding completed'}: ${new Date().toISOString()}`,
    `- ${isVi ? 'Người tạo' : 'Creator'}: ${answers.about_you ? answers.about_you.split(',')[0].trim() : 'User'}`,
    '',
    `> ${isVi ? 'Tự động tạo từ Welcome Onboarding' : 'Auto-generated from Welcome Onboarding'}`,
  ].join('\n');
}
