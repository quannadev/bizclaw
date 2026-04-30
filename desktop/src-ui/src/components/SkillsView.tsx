import { useStore } from '../stores/appStore'

export function SkillsView() {
  const { skills, installSkill, uninstallSkill } = useStore()

  const installedSkills = skills.filter(s => s.installed)
  const availableSkills = skills.filter(s => !s.installed)

  return (
    <div class="skills-view">
      <h2>Skills</h2>
      <p class="subtitle">Extend your AI agent's capabilities</p>

      {installedSkills.length > 0 && (
        <section class="skills-section">
          <h3>Installed Skills</h3>
          <div class="skills-grid">
            {installedSkills.map(skill => (
              <div key={skill.id} class="skill-card installed">
                <h4>{skill.name}</h4>
                <p>{skill.description}</p>
                <button 
                  class="btn-uninstall"
                  onClick={() => uninstallSkill(skill.id)}
                >
                  Uninstall
                </button>
              </div>
            ))}
          </div>
        </section>
      )}

      <section class="skills-section">
        <h3>Available Skills</h3>
        <div class="skills-grid">
          {availableSkills.map(skill => (
            <div key={skill.id} class="skill-card">
              <h4>{skill.name}</h4>
              <p>{skill.description}</p>
              <button 
                class="btn-install"
                onClick={() => installSkill(skill.id)}
              >
                Install
              </button>
            </div>
          ))}
        </div>
      </section>

      <section class="skills-section">
        <h3>Browse Marketplace</h3>
        <p>Visit the online marketplace to discover more skills</p>
        <button class="btn-marketplace">Open Marketplace</button>
      </section>
    </div>
  )
}
